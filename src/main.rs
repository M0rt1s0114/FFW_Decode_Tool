use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use indicatif::{ProgressBar, ProgressStyle};

// ---------- 核心算法 ----------
fn ror(val: u8, r: usize) -> u8 {
    let r = r % 8;
    let v = val as u16;
    ((v >> r) | (v << (8 - r))) as u8
}

fn rol(val: u8, r: usize) -> u8 {
    let r = r % 8;
    let v = val as u16;
    ((v << r) | (v >> (8 - r))) as u8
}

fn compute_mask(i: usize) -> u8 {
    let mask = (1u32 << (i & 3))
        ^ (1u32 << (i % 7))
        ^ (1u32 << ((i % 13) + 4));
    (mask & 0xFF) as u8
}

// 编码（加密）
pub fn encode(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    for (i, &byte) in data.iter().enumerate() {
        let r = i % 8;
        let mask = compute_mask(i);
        let tmp = byte ^ mask;
        out.push(rol(tmp, r));
    }
    out
}

// 解码（解密）
pub fn decode(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    for (i, &byte) in data.iter().enumerate() {
        let r = i % 8;
        let tmp = ror(byte, r);
        let mask = compute_mask(i);
        out.push(tmp ^ mask);
    }
    out
}

// ---------- 命令行模式 ----------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Encode, // -d 加密
    Decode, // -e 解密
}

/// 显示帮助信息（完全按照用户要求）
fn print_help() {
    // 标题和用法行
    println!("SM232X FFW文件加解密实用工具");
    println!("用法: sm232x_ffwtool [选项] <输入文件> [输出文件]");
    println!("选项:");
    println!("  -e             解密FFW");
    println!("  -d             加密FFW");
    println!("  -h, --help     显示此帮助信息");
    println!();
    println!("示例:");
    println!("  sm232x_ffwtool -e firmware.ffw firmware_decoded.ffw");
    println!("  sm232x_ffwtool -d firmware_decoded.ffw firmware_encoded.ffw");
}

/// 解析命令行参数
fn parse_args(args: &[String]) -> Result<(Mode, PathBuf, Option<PathBuf>), String> {
    let mut mode = None;
    let mut input = None;
    let mut output = None;
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "-h" | "--help" => return Err("help".to_string()), // 触发显示帮助
            "-e" => {
                if mode.is_some() {
                    return Err("选项 -e 和 -d 不能同时使用".to_string());
                }
                mode = Some(Mode::Decode);
            }
            "-d" => {
                if mode.is_some() {
                    return Err("选项 -e 和 -d 不能同时使用".to_string());
                }
                mode = Some(Mode::Encode);
            }
            _ if arg.starts_with('-') => {
                return Err(format!("未知选项: {}", arg));
            }
            _ => {
                // 普通参数（文件路径）
                if input.is_none() {
                    input = Some(PathBuf::from(arg));
                } else if output.is_none() {
                    output = Some(PathBuf::from(arg));
                } else {
                    return Err("提供了多余的参数".to_string());
                }
            }
        }
        i += 1;
    }

    let mode = mode.ok_or_else(|| "请指定模式：-e（解密）或 -d（加密）".to_string())?;
    let input = input.ok_or_else(|| "请指定输入文件".to_string())?;

    Ok((mode, input, output))
}

/// 生成默认输出文件名（与输入文件同目录）
fn default_output_path(input: &Path, mode: Mode) -> PathBuf {
    let dir = input.parent().unwrap_or_else(|| Path::new(""));
    let file_name = input.file_name().unwrap_or_default().to_string_lossy();
    let suffix = match mode {
        Mode::Encode => "_encoded",
        Mode::Decode => "_decoded",
    };
    let new_file_name = format!("{}{}", file_name, suffix);
    dir.join(new_file_name)
}

/// 带进度条的文件处理
fn process_file_with_progress(
    input_path: &Path,
    output_path: &Path,
    mode: Mode,
) -> io::Result<()> {
    let data = fs::read(input_path)?;
    let total = data.len();

    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message("处理中...");

    let result: Vec<u8> = match mode {
        Mode::Encode => {
            let mut out = Vec::with_capacity(total);
            for (i, &byte) in data.iter().enumerate() {
                let r = i % 8;
                let mask = compute_mask(i);
                let tmp = byte ^ mask;
                out.push(rol(tmp, r));
                pb.inc(1);
            }
            out
        }
        Mode::Decode => {
            let mut out = Vec::with_capacity(total);
            for (i, &byte) in data.iter().enumerate() {
                let r = i % 8;
                let tmp = ror(byte, r);
                let mask = compute_mask(i);
                out.push(tmp ^ mask);
                pb.inc(1);
            }
            out
        }
    };

    pb.finish_with_message("处理完成");
    fs::write(output_path, result)?;
    Ok(())
}

// ---------- 主函数 ----------
fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // 如果没有参数，显示帮助
    if args.len() == 1 {
        print_help();
        return Ok(());
    }

    match parse_args(&args[1..]) {
        Ok((mode, input, output)) => {
            let output_path = output.unwrap_or_else(|| default_output_path(&input, mode));

            if let Err(e) = process_file_with_progress(&input, &output_path, mode) {
                eprintln!("处理失败: {}", e);
                std::process::exit(1);
            }

            println!("输出文件: {}", output_path.display());
            Ok(())
        }
        Err(msg) if msg == "help" => {
            print_help();
            Ok(())
        }
        Err(msg) => {
            eprintln!("错误: {}", msg);
            eprintln!("使用 sm232x_ffwtool -h 查看帮助");
            std::process::exit(1);
        }
    }
}

// ---------- 单元测试 ----------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ror_rol() {
        for val in 0..=255 {
            for r in 0..8 {
                let rotated = ror(val, r);
                assert_eq!(rol(rotated, r), val);
            }
        }
    }

    #[test]
    fn test_mask() {
        for i in 0..1000 {
            let js_mask = ((1u32 << (i & 3)) ^ (1u32 << (i % 7)) ^ (1u32 << ((i % 13) + 4))) & 0xFF;
            assert_eq!(compute_mask(i) as u32, js_mask);
        }
    }

    #[test]
    fn test_encode_decode() {
        let original = b"Hello, world! This is a test.";
        let encoded = encode(original);
        let decoded = decode(&encoded);
        assert_eq!(original, &decoded[..]);
    }
}