// I may have gotten a little carried away with these macros..

macro_rules! try_opt {
    ($value:expr) => {{
        match $value {
            Some(v) => v,
            None    => return None,
        }
    }};
}

macro_rules! gen_value {
    // Option
    ($parent:expr, $disp_name:expr, None) => {{
        ParseYaml::from(&$parent[$disp_name])
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
        ParseYaml::from(&$parent[$disp_name]).unwrap_or($default)
    }};
}

macro_rules! get_default {
    ([$min:expr, $default:expr]) => ($default);
    ($default:expr) => ($default);
}

#[macro_export]
macro_rules! create_config_struct_d {
    ($name:ident, $($field:ident: $field_t:ty => $disp_name:expr => $default:tt,)+) => {
        #[derive(Debug)]
        pub struct $name {
            $(pub $field: $field_t,)+
        }

        impl ParseYaml for $name {
            fn from(doc: &Yaml) -> Option<$name> {
                Some($name {
                    $($field: gen_value!(doc, $disp_name, $default),)+
                })
            }
        }

        impl Default for $name {
            fn default() -> $name {
                $name {
                    $($field: get_default!($default),)+
                }
            }
        }
    };
}

#[macro_export]
macro_rules! create_config_struct {
    ($name:ident, $($field:ident: $field_type:ty => $disp_name:expr,)+) => {
        #[derive(Debug, Default)]
        pub struct $name {
            $(pub $field: $field_type,)+
        }

        impl ParseYaml for $name {
            fn from(doc: &Yaml) -> Option<$name> {
                Some($name {
                    $($field: try_opt!(ParseYaml::from(&doc[$disp_name])),)+
                })
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
                        let value = ParseYaml::from(elem);

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