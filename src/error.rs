error_chain! {
    links {
        Config      (::config::Error,          ::config::ErrorKind);
        Feed        (::feed::Error,            ::feed::ErrorKind);
        Listeners   (::feed::listeners::Error, ::feed::listeners::ErrorKind);
        Notification(::notification::Error,    ::notification::ErrorKind);
        Util        (::util::Error,            ::util::ErrorKind);
    }
}

fn get_err_msg(err: &Error) -> String {
    let causes = err
        .iter()
        .skip(1)
        .map(|e| format!("caused by: {}\n", e))
        .collect::<String>();

    format!("error: {}\n{}", err, causes)
}

/// Displays the provided error with a notification and by writing it to the terminal
pub fn report(err: &Error) {
    let msg = get_err_msg(&err);
    eprintln!("{}", msg);

    if let Some(backtrace) = err.backtrace() {
        eprintln!("{:?}", backtrace);
    }

    match ::notification::create_error(&msg) {
        Ok(_) => (),
        Err(err) => {
            let err = err.into();
            eprintln!("failed to create error notification:\n{}", get_err_msg(&err));

            if let Some(backtrace) = err.backtrace() {
                eprintln!("{:?}", backtrace);
            }
        }
    }
}