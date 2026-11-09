# 설치

## 내려받기

[최신 릴리스](https://github.com/Domiki/STARY/releases)에서 사용하는 시스템에 맞는 파일을 받으세요.

| 시스템 | 파일 |
|---|---|
| macOS, Apple silicon (M1 이상) | `.dmg` |
| Windows 10 / 11, 64비트 | 설치 프로그램 |

Intel Mac과 Linux는 아직 지원하지 않습니다.

## 처음 열 때 { #first-open }

프리릴리스는 유료 개발자 인증서로 서명하지 않아서, 처음 한 번 시스템이 경고를 보여 줍니다.

=== "macOS"

    `.dmg`를 열고 STARY를 **Applications**로 끌어 놓으세요. 앱이 열리지 않으면 앱을 우클릭해서 **열기**를
    선택하거나, *시스템 설정 → 개인정보 보호 및 보안 → 그래도 열기*에서 허용하세요. "손상되었다"고 나오면
    격리 표시를 지웁니다.

    ```
    xattr -cr /Applications/STARY.app
    ```

=== "Windows"

    설치 프로그램을 실행하세요. SmartScreen이 나타나면 **추가 정보 → 실행**을 선택합니다. WebView2 런타임이
    필요하며 Windows 11에는 기본으로 들어 있습니다.

## 권장 사양

- **메모리**: 24~26메가픽셀 RAW라면 16 GB 이상을 권장합니다. 연산 패널의 *메모리 예산*으로 STARY가 쓰는
  양을 제한할 수 있습니다.
- **GPU**: 선택 사항입니다. Metal, DirectX 12, Vulkan을 지원하는 GPU가 있으면 스태킹이 빨라집니다.
- **디스크**: 26메가픽셀 결과는 약 300 MB(32비트 float TIFF)입니다.

## 소스에서 빌드

Rust(버전은 `rust-toolchain.toml`에 고정)와 Node.js가 필요합니다.

```
git clone https://github.com/Domiki/STARY.git
cd STARY
npm install
npm --prefix ui install
npx tauri build
```

Windows에서는 Visual Studio Build Tools(C++ 워크로드)와 LibRaw를 컴파일할 LLVM도 필요합니다
(`.cargo/config.toml` 참고). 설치 파일은 `target/release/bundle`에 만들어집니다.
