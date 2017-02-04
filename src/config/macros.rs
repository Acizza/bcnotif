use config::yaml_rust::Yaml;

// I may have gotten a little carried away with these macros..

// Due to limitations of the macro system, we must use a generic solution
// to retrieve values dynamically.
pub fn yaml_to_string(yaml: &Yaml) -> Option<String> {
    use config::yaml_rust::Yaml::*;

    match *yaml {
        Real(ref string) | String(ref string) =>
            Some(string.clone()),
        Integer(num) => Some(format!("{}", num)),
        _ => None,
    }
}

macro_rules! yaml_to_string {
    ($yaml:expr) => ($crate::config::macros::yaml_to_string($yaml));
}

macro_rules! try_opt {
    ($value:expr) => {{
        match $value {
            Some(v) => v,
            None    => return None,
        }
    }};
}

#[macro_export]
macro_rules! gen_value {
    // Option
    ($parent:expr, $disp_name:expr, None) => {{
        yaml_to_string!(&$parent[$disp_name])
            .and_then(|s| s.parse().ok())
    }};

    // Option with minimum
    ($parent:expr, $disp_name:expr, [$min:expr, None]) => {{
        let result = gen_value!($parent, $disp_name, None);
        result.map(|v| if v < $min { $min } else { v })
    }};

    // Value with minimum
    ($parent:expr, $disp_name:expr, [$min:expr, $default:expr]) => {{
        let result = gen_value!($parent, $disp_name, $default);
        if result < $min { $min } else { result }
    }};

    // Value
    ($parent:expr, $disp_name:expr, $default:expr) => {{
        yaml_to_string!(&$parent[$disp_name])
            .and_then(|s| s.parse().ok())
            .unwrap_or($default)
    }};
}

#[macro_export]
macro_rules! create_config_section {
    ($name:ident, $($field:ident: $field_t:ty => $disp_name:expr => $default:tt,)+) => {
        #[derive(Debug)]
        pub struct $name {
            $(pub $field: $field_t,)+
        }

        impl $name {
            pub fn new(doc: &Yaml) -> $name {
                $name {
                    $($field: gen_value!(doc, $disp_name, $default),)+
                }
            }
        }
    };
}

#[macro_export]
macro_rules! create_config_arr {
    ($name:ident, $($field:ident: $field_type:ty => $disp_name:expr,)+) => {
        #[derive(Debug)]
        pub struct $name {
            $(pub $field: $field_type,)+
        }

        impl $name {
            pub fn parse(doc: &Yaml) -> Vec<$name> {
                doc.as_vec()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .filter_map(|field| {
                        Some($name {
                            $($field:
                                try_opt!(
                                    yaml_to_string!(&field[$disp_name])
                                    .and_then(|s| s.parse().ok())
                                ),)+
                        })
                    })
                    .collect()
            }
        }
    };
}

#[macro_export]
macro_rules! create_config_enum {
    ($name:ident, $($field:ident($field_t:ty) => $disp_name:expr,)+) => {
        #[derive(Debug)]
        pub enum $name {
            $($field($field_t),)+
        }

        impl $name {
            pub fn parse(doc: &Yaml) -> Option<Vec<$name>> {
                let elements   = try_opt!(doc.as_vec());
                let mut values = Vec::new();

                for element in elements {
                    $(
                    let elem = &element[$disp_name];

                    if !elem.is_badvalue() {
                        let value = yaml_to_string!(elem)
                                        .and_then(|s| s.parse().ok());

                        match value {
                            Some(v) => {
                                values.push($name::$field(v));
                                continue
                            },
                            None => (),
                        }
                    }
                    )+
                }

                Some(values)
            }
        }
    };
}