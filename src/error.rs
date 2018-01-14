use failure::Error;
use notify;

fn build_err_msg(err: &Error) -> String {
    let mut msg = format!("error: {}\n", err.cause());

    for cause in err.causes().skip(1) {
        msg.push_str(&format!("caused by: {}\n", cause));
    }

    msg
}

fn print_with_backtrace(msg: &str, err: &Error) {
    eprintln!("{}", msg);
    eprintln!("{}", err.backtrace());
}

/// Displays the provided error with a notification and by writing it to the terminal
pub fn display(err: &Error) {
    let msg = build_err_msg(err);
    print_with_backtrace(&msg, err);

    match notify::create_error(&msg) {
        Ok(_) => (),
        Err(notif_err) => {
            eprintln!("failed to create error notification:");

            let notif_msg = build_err_msg(&notif_err);
            print_with_backtrace(&notif_msg, &notif_err);
        }
    }
}
