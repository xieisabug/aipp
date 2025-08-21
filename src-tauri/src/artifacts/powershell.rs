use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

pub fn run_powershell(script: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut child = Command::new("powershell")
        .args(&["-NoExit", "-Command", script])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let (tx, rx) = mpsc::channel();

    // 读取标准输出
    let tx_stdout = tx.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            tx_stdout.send(format!("stdout: {}", line.unwrap())).unwrap();
        }
    });

    // 读取标准错误
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            tx.send(format!("stderr: {}", line.unwrap())).unwrap();
        }
    });

    let mut output = String::new();

    // 持续读取输出，直到进程结束
    loop {
        match rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(line) => {
                println!("{}", line); // 实时打印输出
                output.push_str(&line);
                output.push('\n');
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // 检查进程是否仍在运行
                match child.try_wait() {
                    Ok(Some(status)) => {
                        output.push_str(&format!("PowerShell 执行完成，退出状态: {}", status));
                        break;
                    }
                    Ok(None) => continue, // 进程仍在运行
                    Err(e) => return Err(Box::new(e)),
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    // 确保子进程已经完全退出
    let _ = child.wait_with_output()?;

    Ok(output)
}
