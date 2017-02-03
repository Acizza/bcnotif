macro_rules! report_error {
    ($func:path, $err:expr) => {{
        let err_str = &format!("{:?}", $err);

        match $crate::notification::create_error(err_str) {
            Ok(_)    => (),
            Err(err) => println!("error creating error notification: {:?}", err),
        }

        $func!("error: {}", err_str);
    }}
}

#[macro_export]
macro_rules! check_err {
    ($x:expr) => {{
        match $x {
            Ok(_) => (),
            Err(err) => report_error!(println, err),
        }
    }};

    ($x:expr, $default:expr) => {{
        match $x {
            Ok(v) => v,
            Err(err) => {
                report_error!(println, err);
                $default
            },
        }
    }};
}

#[macro_export]
macro_rules! check_err_p {
    ($x:expr) => {{
        match $x {
            Ok(v) => v,
            Err(err) => report_error!(panic, err),
        }
    }};
}

#[macro_export]
macro_rules! check_err_c {
    ($x:expr, $additional_code:block) => {{
        match $x {
            Ok(v) => v,
            Err(err) => {
                report_error!(println, err);
                $additional_code
                continue
            }
        }
    }};

    ($x:expr) => (check_err_c!($x, {}));
}