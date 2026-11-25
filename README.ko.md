[English](README.md) | **한국어** | [日本語](README.ja.md) | [简体中文](README.zh-CN.md)

# bmo-artifact

문서를 보관하고 검색하고 공유하는, 자체 API를 갖춘 하나의 서비스입니다. 기록, 학습 책, 가져온 웹 페이지와 HTML
페이지를 다루며, 각 문서는 파일, 참조하는 페이지, 공유 링크, 그리고 읽도록 공유된 읽는 사람의 회사 구성원을
갖습니다. 하네스는 MCP 서버 `bmo-artifact mcp`와 `bmo-artifact` 스킬로, 그 밖의 호출자는 HTTP API와 Rust
클라이언트로 접근합니다. 웹 페이지는 bmo-explorer의 클라이언트로 읽으며, 다른 유닛은 호출하지 않습니다.

## 구성

| 폴더 | 내용 |
|---|---|
| `src/` | 라이브러리와 서비스 바이너리 `bmo-artifact`: 라우트, 저장소, 클라이언트, MCP 서버, 명령 |
| `tools/` | 도구 호스트가 배치하는 도구: 매니페스트, 스키마, 실행 파일 `artifact-tool` |
| `lang/` | 문서 문구 파일의 영어 기본값 |
| `skills/` | `bmo-artifact` 스킬 |
| `client/` | 콘솔 번들 |
| `tests/` | 저장소 검사를 포함한 테스트 |
| `deploy/` | 중립적인 서비스 유닛 템플릿과 배포가 게시하는 앱 선언 |
| `docs/` | 서비스 동작에 대한 기준 문서 |

루트에는 Cargo 매니페스트, `testbed.toml`, `install.toml`, `rustfmt.toml`, `CLAUDE.md`,
`PERSONAL-DATA.md`, `LICENSE`, `settings.toml`, 그리고 배포 시 주는 로컬 설정의 예시인
`settings.local.toml.example`이 있습니다.

## 문서

| 문서 | 내용 |
|---|---|
| `docs/api.md` | API: 라우트, 이벤트, 명령, 클라이언트, MCP 도구, bmo-explorer 호출 |
| `docs/openapi.json` | 코드에서 생성한 OpenAPI 3.1 문서 형태의 API |
| `docs/artifact.md` | 문서의 설계. API가 이를 다루고 구성이 이를 배치함 |
| `docs/layout.md` | 구성, 배관, 콘솔 번들, 저장 방식, 버전, 선언, 검사 |
| `PERSONAL-DATA.md` | 어떤 파일에도 들어가면 안 되는 개인정보와 이를 지키는 검사 |

## 버전

- **버전은 세트 전체의 것이며,** 빌드할 때 git 태그에서 읽습니다. 헬스 라우트, MCP 서버, `--version`이 이 버전으로
  응답하고, 크레이트와 콘솔 번들 패키지의 버전은 움직이지 않는 자리표시 값입니다.

## 명령

```sh
cargo check --all-targets
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run --bin bmo-artifact -- serve
cargo run --bin bmo-artifact -- mcp
cargo run --bin bmo-artifact -- openapi
```
