use crate::error::NotifyError;

#[cfg(any(unix, macos))]
mod unix {
    extern crate notify_rust;

    use self::notify_rust::Notification;
    use super::*;

    pub fn create(title: &str, body: &str) -> Result<(), NotifyError> {
        Notification::new()
            .summary(title)
            .body(body)
            .show()
            .map_err(|_| NotifyError::CreationFailed)?;

        Ok(())
    }
}

#[cfg(windows)]
mod windows {
    use super::NotifyError;
    use winrt::windows::data::xml::dom::*;
    use winrt::windows::ui::notifications::*;
    use winrt::FastHString;

    impl From<::winrt::Error> for NotifyError {
        fn from(err: ::winrt::Error) -> NotifyError {
            NotifyError::WinRT(err)
        }
    }

    // https://stackoverflow.com/a/46817674
    //
    // The Toast Notification Manager needs a valid app ID for any notifications to actually display,
    // so we'll use one that is already defined since it is not worth the effort to create one ourselves.
    const APP_ID: &str =
        "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\WindowsPowerShell\\v1.0\\powershell.exe";

    pub fn create(title: &str, body: &str) -> Result<(), NotifyError> {
        let toast_xml =
            ToastNotificationManager::get_template_content(ToastTemplateType::ToastText02)?
                .ok_or_else(|| NotifyError::NullElement("template content".into()))?;

        let toast_text_elements = toast_xml
            .get_elements_by_tag_name(&FastHString::new("text"))?
            .ok_or_else(|| NotifyError::NullElement("text elements".into()))?;

        let add_text = |i, string| {
            let node = &*toast_xml
                .create_text_node(&FastHString::new(string))?
                .ok_or_else(|| NotifyError::NullElement("text node".into()))?
                .query_interface::<IXmlNode>()
                .ok_or_else(|| NotifyError::NullElement("query interface".into()))?;

            toast_text_elements
                .item(i)?
                .ok_or_else(|| NotifyError::NullElement("text item".into()))?
                .append_child(node)?
                .ok_or_else(|| NotifyError::NullElement("child node".into()))
        };

        add_text(0, title)?;
        add_text(1, body)?;

        let toast = ToastNotification::create_toast_notification(&*toast_xml)?;

        ToastNotificationManager::create_toast_notifier_with_id(&FastHString::new(APP_ID))?
            .ok_or_else(|| NotifyError::NullElement("toast notification".into()))?
            .show(&*toast)?;

        Ok(())
    }
}

#[cfg(any(unix, macos))]
pub use self::unix::create;

#[cfg(windows)]
pub use self::windows::create;

pub fn create_error(body: &str) -> Result<(), NotifyError> {
    create("Broadcastify Update Error", body)
}
