# @file: build_rust.ps1
# @description: Compiles the rust_core library using the specific Conda environment.
# @usage: ./build_rust.ps1

# #1. Define the correct Python path
$PYTHON_PATH = "C:\Users\luiss\.conda\envs\institutional_env\python.exe"

# #2. Check if the path exists
if (-Not (Test-Path $PYTHON_PATH)) {
    Write-Host "Error: Python executable not found at $PYTHON_PATH" -ForegroundColor Red
    exit 1
}

Write-Host "Compiling rust_core using: $PYTHON_PATH" -ForegroundColor Cyan

# #3. Run Maturin via the python module (-m)
# --release makes it fast (optimized)
& $PYTHON_PATH -m maturin develop --release

if ($LASTEXITCODE -eq 0) {
    Write-Host "`nBuild Successful! You can now run the Python test script." -ForegroundColor Green
} else {
    Write-Host "`nBuild Failed." -ForegroundColor Red
}