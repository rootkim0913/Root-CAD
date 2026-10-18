# Root CAD 사용 안내

Windows 10/11 64비트 Intel/AMD용입니다. DirectX 12를 지원하는 그래픽 드라이버가 필요합니다. macOS/Linux용 실행 파일이나 ARM64 네이티브 빌드는 이 ZIP에 없습니다.

## 실행

1. ZIP 전체를 쓰기 가능한 폴더에 압축 해제합니다. ZIP 안에서 바로 실행하지 마세요.
2. `Start-RootCAD.cmd`를 더블 클릭합니다. Rust, Git, Python, 별도의 CAD 설치는 필요 없습니다.
3. `examples/root-course-demo.dxf`를 열어 기본 제도 기능을 확인할 수 있습니다.

프로그램은 `root-cad.exe`, 자동화용 CLI는 `root-cad-cli.exe`입니다. 두 파일을 함께 보관하세요. 실행 스크립트는 작업 공간·로그·자동 저장 폴더로 옆의 `data`를 사용합니다. 일부 화면 환경설정은 Windows 사용자 프로필에 저장될 수 있습니다. 종료 전에 작업 도면을 DXF로 저장하세요. 도면 파일을 옮기면 다른 컴퓨터에서도 이어서 편집할 수 있습니다.

## Claude / Codex 연결 (선택)

일반 CAD 기능은 AI 클라이언트 없이 작동합니다. AI 연결은 기존 Claude/Codex 계정과 프로그램이 필요하며, Root CAD에는 별도 API 키를 저장하지 않습니다.

PowerShell에서 이 폴더로 이동한 뒤 다음을 실행하면 **현재 PC의 실제 폴더 경로**를 사용한 설정이 `mcp-config`에 생성됩니다.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\Configure-MCP.ps1
```

Claude에는 `mcp-config/claude.json`의 `root-cad` 항목만 병합합니다. Codex CLI가 설치되어 있다면 다음 명령으로 등록할 수 있습니다.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\Configure-MCP.ps1 -InstallCodex
```

다른 PC나 다른 폴더로 옮긴 후에는 설정을 다시 생성하세요. MCP는 로컬 `127.0.0.1:7980` 포트를 사용합니다. 같은 포트로 이미 실행 중인 CAD 창이 있다면 연결 대상이 그 창이 될 수 있으므로, 여러 버전을 동시에 자동화하지 마세요.

## 지원 범위와 문제 해결

- `D`/`DIMSTYLE`: 치수 스타일, `FIND`: 찾기·바꾸기, `LE`/`QLEADER`: 빠른 지시선.
- 강의별 지원 범위는 `docs/course-compatibility.ko.md`에 있습니다. AutoCAD의 모든 고급 기능·파일 설정을 동일하게 재현하는 제품은 아닙니다.
- 창이 뜨지 않으면 Windows와 그래픽 드라이버를 업데이트하고 `data/logs`를 확인하세요. 한글 UI는 Windows의 맑은 고딕 등 설치된 한글 글꼴을 사용합니다.
- 이 배포본은 코드 서명 인증서를 사용하지 않은 초기 배포입니다. Windows에서 게시자를 확인할 수 없다는 표시가 나올 수 있습니다. 릴리스의 SHA256SUMS.txt로 다운로드 파일의 무결성을 확인할 수 있습니다.
- 현재 개발 PC와 개발 도구 경로를 제외한 이동 폴더에서 실행을 검증했습니다. 모든 종류의 하드웨어나 별도의 실제 PC/가상 머신에서 검증한 것은 아닙니다.

## 라이선스

Root CAD는 CADCraft 기반의 수정 배포입니다. 원본 저작권과 기여자 표시는 유지합니다. 프로젝트 라이선스는 MIT OR Apache-2.0이며, `NOTICE`, `ATTRIBUTION.md`, `LICENSE-MIT`, `LICENSE-APACHE`를 참고하세요. 의존성 목록과 해당 라이선스는 `THIRD-PARTY-NOTICES.txt`, `licenses`에 포함합니다. 시스템 글꼴과 Windows DLL을 복제해 배포하지 않습니다.
