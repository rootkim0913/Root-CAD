# 대화로 2D 도면 만들기

Root CAD의 로컬 MCP 서버를 기존 Claude 또는 Codex에 연결합니다. Root CAD에는 API 키를
저장하지 않습니다. AI 서비스 이용은 연결한 클라이언트의 계정과 설정을 따릅니다.
음성 인식은 AI 클라이언트 또는 Windows 음성 입력(Win+H)을 사용합니다.
Root CAD 자체의 녹음 기능이나 휴대폰 원격 접속 서버가 추가된 것은 아닙니다.

## Windows에서 실행

프로젝트 폴더에서 PowerShell로 실행합니다.

```powershell
.\scripts\Build-AiCad.ps1
.\scripts\Start-AiCad.ps1
```

빌드는 Rust가 필요합니다. 프로젝트 내부 `.tools`에 설치한 Rust가 있으면 이를 사용합니다.
빌드가 끝나면 `dist/mcp-claude.json`, `dist/mcp-codex.toml`에 이 PC의 실행 파일 경로를 포함한
설정이 생성됩니다. 각각 기존 클라이언트의 MCP 설정에 **cadcraft 항목만 병합**하고
클라이언트를 다시 시작합니다. 다른 서버 설정을 덮어쓰지 마세요.
Codex CLI에서는 다음 명령으로도 등록할 수 있습니다(프로젝트 폴더에서 실행).

설정 형식은 [공식 Codex MCP 문서](https://learn.chatgpt.com/docs/extend/mcp?surface=cli)를 따릅니다.

```powershell
codex mcp add cadcraft -- "$PWD\target\debug\root-cad-cli.exe" mcp --connect 127.0.0.1:7980 --launch
```

새 GUI와 CLI는 같은 폴더에 있어야 합니다. 첫 MCP 호출에서 GUI가 열리므로 별도로 실행할
필요도 없습니다. 기존 `C:\Program Files\CADCraft` 설치본에는 새 명령이 없습니다.
기존 설치본과 충돌을 피하도록 새 빌드는 7980 포트를 사용합니다. 연결은 로컬 PC만 사용합니다.

## 플랜지 예제

AI에게 다음처럼 요청할 수 있습니다.

> 새 밀리미터 도면을 만들고 외경 100, 파이프 외경 70, 길이 500,
> PCD 85, 지름 10인 홀 12개의 플랜지를 그려줘. 빠진 치수는 알려줘.

AI는 `new_drawing {"metric":true}` 후 `create_flange`를 호출할 수 있습니다.

```json
{"outerDiameter":100,"pipeDiameter":70,"pipeLength":500,"boltCircle":85,"holeDiameter":10,"holes":12}
```

정면도, 파이프 외경, 홀, 중심선을 만듭니다. 플랜지 두께를 입력하지 않았으므로 측면도는
생략하고 경고합니다. `flangeThickness:12`를 추가하면 길이 500의 파이프와 플랜지의
개략 측면 외곽선도 만듭니다. 내경·관 두께·공차·재질은 추정하지 않으며 제조 규격 검증은
별도로 필요합니다. Root CAD의 모든 기본 편집 명령으로 생성된 객체를 수정할 수 있습니다.

## 기존 도면 수정

`inspect_drawing`, `query_entities`, `find_text`로 대상 핸들을 확인하고,
여러 수정은 `draft_plan`으로 묶습니다.

```json
{"dryRun":true,"steps":[
  {"command":"circle","params":{"center":[0,0],"radius":50}},
  {"command":"circle","params":{"center":[0,0],"radius":35}}
]}
```

`dryRun:true`는 실제 실행으로 검증한 뒤 도면과 이력을 복원합니다. 이때 반환한 핸들은
임시 값입니다. 확인 후 같은 계획을 `dryRun:false`로 적용합니다. 중간 명령이 실패하면
전체를 되돌리고, 성공한 계획은 UNDO 한 번으로 취소합니다. 파일 저장·문서 전환·스크립트·
중첩 계획은 묶음 안에서 실행할 수 없습니다. 허용 명령은 `cmd/drafting.rs`의 명시적 목록을
따릅니다. 새로 만든 객체의 핸들이 필요한 후속 작업은 첫 계획 결과를 받은 뒤 별도 계획으로
실행합니다. 마지막에 `render`로 결과를 확인하고 요청한 경로에 `save`합니다.

## PDF 도면 가져오기

`import_pdf`에 파일 경로와 페이지 번호(1부터)를 전달합니다.

```json
{"path":"C:/drawings/part.pdf","page":1}
```

벡터 경로를 `PDF_VECTOR` 레이어의 편집 가능한 폴리선으로 읽습니다. 기본 길이는
**종이에 출력된 mm**입니다. 모델의 실제 크기는 알고 있는 치수로 보정해야 합니다.
다음은 PDF에서 50 mm 떨어진 두 점이 실제로는 1000인 경우입니다.

```json
{"path":"C:/drawings/part.pdf","page":1,
 "reference":{"from":[0,0],"to":[50,0],"length":1000},"at":[0,0]}
```

축척은 20이 됩니다. `reference` 대신 `scale:20`도 가능합니다. 보정 기준점은 가져오기 전의
종이 mm 좌표이며 `at`은 보정 후 도면의 삽입점입니다. 기존 메뉴의 파일 열기로도 PDF
첫 페이지를 읽을 수 있고, 원본 PDF를 QSAVE로 덮어쓰지 않습니다.

범위: 선, 사각형, 베지어 곡선(32개 직선 조각), 변환 행렬, 중첩 Form XObject.
문자·치수 의미·래스터 이미지·클리핑·색/선가중치·페이지 회전/크롭/UserUnit은 복원하지 않습니다.
일부 경로가 누락되거나 클리핑 바깥의 선이 나타날 수 있으므로 `warnings`와 렌더 결과를
확인해야 합니다. 스캔 PDF의 자동 벡터화/OCR은 포함되지 않습니다. 스캔은 AI 클라이언트에
직접 보여주고 알려진 치수와 추정 치수를 구분해 재작성할 수 있습니다.

참고 영상: [카우스 소거법의 AI 캐드 소개](https://www.youtube.com/watch?v=vi42ztaCKw0).
영상 설명의 PDF 변환·기존 도면 수정·플랜지 생성 흐름을 참고했으며, 영상 속 제품의 코드나
자산을 복제하지 않았습니다. 영상의 12초 완료 시간은 보장하지 않습니다.
