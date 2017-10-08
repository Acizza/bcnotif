use error_chain::ChainedError;
use notify;

fn get_err_msg<T: ChainedError>(err: &T) -> String {
    let causes = err
        .iter()
        .skip(1)
        .map(|e| format!("source: {}\n", e))
        .collect::<String>();

    format!("error: {}\n{}", err, causes)
}

fn print_backtrace<T: ChainedError>(err: &T) {
    if let Some(backtrace) = err.backtrace() {
        eprintln!("{:?}", backtrace);
    }
}

/// Displays the provided error with a notification and by writing it to the terminal
pub fn display<T: ChainedError>(err: &T) {
    let msg = get_err_msg(err);
    eprintln!("{}", msg);

    print_backtrace(err);

    match notify::create_error(&msg) {
        Ok(_) => (),
        Err(err) => {
            eprintln!("failed to create error notification:\n{}",
                get_err_msg(&err));

            print_backtrace(&err);
        }
    }
}