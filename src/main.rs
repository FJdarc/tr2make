use serde::Deserialize;
use serde_yaml::Number;
use std::{fs, path::Path};
use clap::{Parser, ValueEnum};

#[derive(Deserialize)]
struct Config {
    language: String,
    standard: Number,
    files: Vec<String>,
    target: String,
    architecture: String,
    model: BuildModels,
}

#[derive(Deserialize)]
struct BuildModels {
    debug: BuildConfig,
    release: BuildConfig,
}

#[derive(Deserialize)]
struct BuildConfig {
    targetdir: String,
}

#[derive(Parser)]
#[command(version, about = r#"
 _____      ____   __  __         _         
|_   _|_ __|___ \ |  \/  |  __ _ | | __ ___ 
  | | | '__| __) || |\/| | / _` || |/ // _ \
  | | | |   / __/ | |  | || (_| ||   <|  __/
  |_| |_|  |_____||_|  |_| \__,_||_|\_\\___|"#, long_about = None)]
struct Args {
    #[arg(index = 1, short, long, default_value = "debug")]
    model: BuildModel,
}

#[derive(Clone, ValueEnum)]
enum BuildModel {
    Debug,
    Release,
}

impl BuildModel {
    fn as_str(&self) -> &'static str {
        match self {
            BuildModel::Debug => "debug",
            BuildModel::Release => "release",
        }
    }
}

fn main() {
    let config: Config = serde_yaml::from_reader(fs::File::open(".tr2make").unwrap()).unwrap();
    let args = Args::parse();

    let build_dir = format!("build/{}-{}", args.model.as_str(), config.architecture);
    fs::create_dir_all(&build_dir).unwrap();

    let (file_ext, compiler, std_prefix) = match config.language.as_str() {
        "c" => (".c", "gcc", "c"),
        "c++" => (".cpp", "g++", "c++"),
        _ => panic!("Unsupported language"),
    };

    let files: Vec<_> = config.files
        .iter()
        .filter(|f| f.ends_with(file_ext))
        .cloned()
        .collect();

    if files.is_empty() {
        panic!("No valid {} files found", file_ext);
    }

    let (target, clean_cmd, mkdir_cmd) = if cfg!(target_os = "windows") {
        (
            format!("{}.exe", config.target),
            "del /Q $(OBJ) $(TARGET)",
            "@if not exist \"$(OBJ_DIR)\" mkdir \"$(OBJ_DIR)\""
        )
    } else {
        (
            config.target.clone(),
            "rm -f $(OBJ) $(TARGET)",
            "@mkdir -p $(OBJ_DIR)"
        )
    };

    let march = match config.architecture.as_str() {
        "x64" => "-m64",
        "x86" => "-m32",
        arch => &format!("-m{arch}")[..],
    };

    let debug_flag = if matches!(args.model, BuildModel::Debug) { "1" } else { "0" };
    let optimization = if debug_flag == "1" { "-g -O0 -DDEBUG" } else { "-O2" };

    let makefile_content = format!(r#"# {lang} Project Makefile
CC := {compiler}
SRC := {files}
TARGET := {target_dir}/{target}
STD := {std_prefix}{standard}
ARCH := {arch}
DEBUG := {debug}

OBJ_DIR := {target_dir}/obj
OBJ := $(addprefix $(OBJ_DIR)/, $(SRC:{file_ext}=.o))

CFLAGS := {optimization} -std=$(STD) {march}
LDFLAGS := {march}

all: create_dirs $(TARGET)

$(TARGET): $(OBJ)
	$(CC) $^ -o $@ $(LDFLAGS)

$(OBJ_DIR)/%.o: %{file_ext}
	{mkdir_cmd}
	$(CC) $(CFLAGS) -c $< -o $@

create_dirs:
	{mkdir_cmd}

clean:
	{clean_cmd}

.PHONY: all clean create_dirs
"#,
        lang = config.language.to_uppercase(),
        compiler = compiler,
        files = files.join(" "),
        target_dir = build_dir,
        target = target,
        std_prefix = std_prefix,
        standard = config.standard,
        arch = config.architecture,
        debug = debug_flag,
        file_ext = file_ext,
        optimization = optimization,
        march = march,
        mkdir_cmd = mkdir_cmd,
        clean_cmd = clean_cmd
    );

    let makefile_path = Path::new(&build_dir).join("Makefile");
    fs::write(&makefile_path, makefile_content).unwrap();

    println!("Makefile generated at: {}", makefile_path.display());
}