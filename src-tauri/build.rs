fn main() {
    // 声明对前端资源目录的依赖，确保前端文件变化时重新构建
    println!("cargo:rerun-if-changed=../src");
    tauri_build::build()
}