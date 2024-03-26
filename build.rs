use std::collections::HashMap;
use std::io::Write;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Config {
    name: String,
    import_offsets: bool,
    import_memory_signatures: bool,
    import_function_signatures: bool,
    console: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct OffsetsAndSignatures {
    offsets: Option<HashMap<String, String>>,
    memory_signature: Option<HashMap<String, String>>,
    function_signatures: Option<HashMap<String, FunctionSignature>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct FunctionSignature {
    arguments: Vec<(String, String)>,
    #[serde(rename = "return")]
    of_return: String,
}

impl FunctionSignature {
    pub fn signature(&self, name: &str) -> String {
        format!(
            "pub type {} = unsafe extern \"fastcall\" fn({}) -> {};",
            name,
            {
                if self.arguments.len() == 0 {
                    String::new()
                } else {
                    self.arguments
                        .iter()
                        .map(|x| {
                            format!(
                                "{}: {}",
                                x.0,
                                validate_type(&x.1).expect(
                                    format!("Invalid type '{}' for '{}' in '{}'", x.1, x.0, name)
                                        .as_str()
                                )
                            )
                        })
                        .collect::<Vec<String>>()
                        .join(", ")
                }
            },
            validate_type(&self.of_return).expect(
                format!("Invalid return type '{}' for '{}'", self.of_return, name).as_str()
            )
        )
    }
}

fn main() {
    // Config Parsing
    let cfg = std::fs::read_to_string("./config.json").expect("Unable to parse Configuration");
    let cfg: Config = serde_json::from_str(&cfg).expect("Unable to parse Configuration");

    let offsets = std::fs::read_to_string("./imports.json").expect("Unable to open offsets file.");
    let offsets: OffsetsAndSignatures =
        serde_json::from_str(&offsets).expect("Unable to parse offsets file.");

    // Offset Parsing
    if cfg.import_offsets {
        let mut output = String::from(
            "#![allow(dead_code, non_upper_case_globals, non_snake_case)]\n\n// Auto-generated:\n",
        );
        for (name, offset) in offsets.offsets.unwrap().iter() {
            let offset: u64 = u64::from_str_radix(&offset.trim_start_matches("0x"), 16).expect(
                &format!("Offset '{}' does not have a valid hexadecimal value.", name),
            );
            output = format!("{}pub const {}: isize = 0x{:x};\n", output, name, offset);
        }
        let mut f =
            std::fs::File::create("./src/offsets.rs").expect("Unable to create offsets file.");
        f.write_all(&output.as_bytes())
            .expect("Error saving Offsets");
    }

    if cfg.import_memory_signatures {
        let mut output = String::from(
            "#![allow(dead_code, non_upper_case_globals, non_snake_case)]\n\n// Auto-generated:\n",
        );

        for (name, signature) in offsets.memory_signature.unwrap().iter() {
            let signature = parse_signature(&signature);
            output = format!(
                "{}pub const {}: [Option<u8>; {}] = [{}];",
                output,
                name,
                signature.len(),
                signature_to_str(signature).join(", ")
            );
        }

        let mut f = std::fs::File::create("./src/memory_signatures.rs")
            .expect("Unable to create offsets file.");
        f.write_all(&output.as_bytes())
            .expect("Error saving Offsets");
    }

    if cfg.import_function_signatures {
        let mut output = String::from(
            "#![allow(dead_code, non_upper_case_globals, non_snake_case)]\n\n// Auto-generated:\n",
        );

        for (name, signature) in offsets.function_signatures.unwrap().iter() {
            output = format!("{}{}\n", output, signature.signature(name));
        }

        let mut f = std::fs::File::create("./src/function_signatures.rs")
            .expect("Unable to create offsets file.");
        f.write_all(&output.as_bytes())
            .expect("Error saving Offsets");
    }

    let mut f = std::fs::File::create("./src/config.rs").expect("Unable to create config file.");
    f.write_all(
        format!(
            "pub const NAME: &str = \"{}\";\npub const CONSOLE: bool = {:?};",
            cfg.name, cfg.console
        )
        .as_bytes(),
    )
    .expect("Error saving Offsets");

    println!("cargo:rerun-if-changed=config.json");
    println!("cargo:rerun-if-changed=imports.json");
    println!("cargo:rerun-if-changed=build.rs");
}

pub fn validate_type(of_type: &str) -> Option<String> {
    let mut of_type = of_type;
    let mut is_ptr = false;
    if of_type.starts_with("*") {
        is_ptr = true;
        of_type = of_type.strip_prefix("*").unwrap();
    }
    let of_type = match of_type.to_lowercase().as_str() {
        "void" => String::from("winapi::ctypes::c_void"),
        "char" => String::from("winapi::ctypes::c_char"),
        "schar" => String::from("winapi::ctypes::c_schar"),
        "uchar" => String::from("winapi::ctypes::c_uchar"),
        "short" => String::from("winapi::ctypes::c_short"),
        "ushort" => String::from("winapi::ctypes::c_ushort"),
        "int" => String::from("winapi::ctypes::c_int"),
        "uint" => String::from("winapi::ctypes::c_uint"),
        "long" => String::from("winapi::ctypes::c_long"),
        "ulong" => String::from("winapi::ctypes::c_ulong"),
        "longlong" => String::from("winapi::ctypes::c_longlong"),
        "ulonglong" => String::from("winapi::ctypes::c_ulonglong"),
        "float" => String::from("winapi::ctypes::c_float"),
        "double" => String::from("winapi::ctypes::c_double"),
        "i8" => String::from("i8"),
        "u8" => String::from("u8"),
        "i16" => String::from("i16"),
        "u16" => String::from("u16"),
        "i32" => String::from("i32"),
        "u32" => String::from("u32"),
        "i64" => String::from("i64"),
        "u64" => String::from("u64"),
        _ => String::new(),
    };

    if of_type.is_empty() {
        None
    } else {
        if is_ptr {
            Some(format!("*mut {}", of_type))
        } else {
            Some(of_type)
        }
    }
}

fn parse_signature(signature: &str) -> Vec<Option<u8>> {
    let signature: Vec<&str> = signature.split(" ").collect::<_>();
    let mut parsed: Vec<Option<u8>> = Vec::new();
    for byte in signature {
        if byte == "?" || byte == "??" {
            parsed.push(None);
            continue;
        }
        match u8::from_str_radix(byte, 16) {
            Ok(n) => parsed.push(Some(n)),
            Err(_err) => parsed.push(None),
        }
    }
    parsed
}

fn signature_to_str(signature: Vec<Option<u8>>) -> Vec<String> {
    let mut res: Vec<String> = Vec::new();

    for byte in signature {
        match byte {
            Some(b) => res.push(format!("Some(0x{:02X})", b)),
            None => res.push(String::from("None")),
        }
    }

    res
}
