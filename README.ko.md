# Claude Token Monitor

[English](README.md) | **한국어**

> 터미널에서 `/usage` 를 치지 않고도 **Claude Code** 토큰 사용 한도가 얼마나 남았는지
> 항상 위에 떠 있는 작은 데스크톱 위젯으로 한눈에 확인하세요.

[![CI](https://github.com/no1fc/claude-token-monitor/actions/workflows/ci.yml/badge.svg)](https://github.com/no1fc/claude-token-monitor/actions/workflows/ci.yml)
![platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-blue)
![stack](https://img.shields.io/badge/built%20with-Tauri%20v2%20%2B%20Rust-orange)
![license](https://img.shields.io/badge/license-MIT-green)

크로스플랫폼 앱으로 **Tauri v2**(Rust 백엔드 + 순수 TypeScript 프론트엔드)로 제작했습니다.
배포 결과물은 테두리 없는 드래그 가능한 위젯과 시스템 트레이 아이콘을 갖춘 작은 단일 실행파일(약 5MB)입니다.

> ⚠️ **비공식 커뮤니티 도구입니다.** Anthropic과 제휴/보증 관계가 없습니다.
> [주의사항](#️-라이브-api-관련-주의)을 참고하세요.

---

## ✨ 기능

**5시간 롤링 윈도우**와 **7일(주간) 윈도우** 각각에 대해:

- **사용량 / 잔여량 퍼센트** — 색상 게이지(초록 → 주황 → 빨강)
- **"X시간 Y분 후 초기화" 실시간 카운트다운**

추가로:

- **소진 속도**(토큰/분)와 **한도 도달 예상 시각(ETA)**
- 현재 세션 및 주간 **예상 비용($)**
- **모델별 사용량 분해**(Opus / Sonnet / Haiku …)
- **현재 세션 토큰** 및 **플랜 등급** 배지(Pro / Max 5x / Max 20x)

데이터 출처는 색상 점으로 표시됩니다: **🟢 초록 = 라이브 API**, **🟠 주황 = 로컬 추정**.

위젯은 다른 앱 위에 떠 있고, 원하는 위치로 드래그할 수 있으며, 트레이에서 표시/숨김이 가능합니다.
창 위치와 모든 설정은 재시작 후에도 유지됩니다.

---

## 📦 설치

### 릴리스에서 받기 (권장)

[**Releases**](../../releases) 페이지에서 최신 설치 파일을 받으세요:

| OS | 파일 |
|----|------|
| Windows | `Claude Token Monitor_<버전>_x64-setup.exe` (NSIS) 또는 `..._x64_en-US.msi` |
| macOS | `..._x64.dmg`(Intel), `..._aarch64.dmg`(Apple Silicon) |
| Linux | `..._amd64.AppImage` / `.deb` / `.rpm` |

> 릴리스 빌드는 GitHub Actions가 Windows · macOS · Linux 전부 자동으로 생성합니다.

### 소스에서 빌드

필요 사항:

- **Node.js** 18+
- **Rust** (stable) — <https://rustup.rs> 에서 설치 (Windows는 MSVC 툴체인)
- OS별 Tauri 시스템 의존성 —
  [Tauri 사전 준비 가이드](https://v2.tauri.app/start/prerequisites/) 참고
  (Windows는 WebView2 — Win11 기본 탑재 / Linux는 `webkit2gtk` + `libayatana-appindicator` / macOS는 Xcode CLT)

```bash
git clone <이-저장소-URL>
cd claudeTokenCheckApp
npm install

npm run tauri dev      # 개발 모드 실행
npm run tauri build    # 현재 OS용 설치 파일 생성
```

결과물은 `src-tauri/target/release/bundle/` 에 생성됩니다.

---

## 🕹 사용법

- **이동:** 위젯 헤더의 빈 영역을 드래그
- **⟳** 새로고침 · **⚙** 설정 열기
- **트레이 아이콘:** 좌클릭 = 표시/숨김 토글, 우클릭 메뉴 =
  표시/숨김 · 강제 새로고침 · 설정 · 종료
- **설정:** 새로고침 주기(최소 60초), 라이브 API 토글, 플랜 + 한도 재정의,
  항상 위, **로그인 시 자동 실행**, 투명도, 테마. OS 설정 폴더에 저장됩니다.

### 바로 실행 & 자동 실행 (Windows)

편의용 스크립트는 [`scripts/`](scripts) 폴더에 있습니다:

| 스크립트 | 동작 |
|----------|------|
| `scripts\run.bat` | 더블클릭하면 빌드된 앱을 즉시 실행 (아직 빌드 전이면 dev 모드로 실행) |
| `scripts\enable-autostart.bat` | Windows **시작프로그램** 폴더에 등록해 로그인 시 자동 실행 |
| `scripts\disable-autostart.bat` | 위 시작프로그램 등록 해제 |

**자동 실행 (모든 OS):** **설정 → "Start automatically on system login"** 을 켜세요.
이 방식이 권장되는 크로스플랫폼 방법이며(Windows 레지스트리 / macOS LaunchAgent /
Linux `.desktop` 자동 시작), 설치된 앱에서도 동작합니다. 인앱 토글과 배치 스크립트를
동시에 써도 안전합니다 — 앱은 단일 인스턴스로 동작해 창이 두 개 뜨지 않습니다.

---

## 🧠 동작 원리 (데이터 출처)

이 앱은 **하이브리드** 방식이며, 필요 시 완전히 오프라인으로도 동작합니다:

1. **로컬 JSONL — 항상 동작 (주 데이터/폴백).** `~/.claude/projects/**/*.jsonl`
   트랜스크립트를 파싱하고, `(requestId, message.id)` 로 중복을 제거한 뒤,
   5시간 과금 "블록"을 재구성([ccusage](https://github.com/ryoppippi/ccusage)와 동일한 방식)
   하고 7일 윈도우를 합산합니다. 토큰 수, 비용, 소진 속도, 모델별 데이터는 모두 여기서 나옵니다.
2. **비공식 사용량 API — 베스트 에포트 (정확한 퍼센트용).** 활성화되어 있고 자격 증명이
   있으면, Claude Code의 `/usage` 가 사용하는 엔드포인트를 호출해 권위 있는 사용 퍼센트와
   초기화 시각을 받아 추정치 위에 덮어씁니다.

로컬 추정에 쓰이는 한도는 **근사값**이며 설정에서 **사용자가 직접 재정의**할 수 있습니다 —
본인의 `/usage` 출력과 비교해 보정하세요.

---

## 🔒 보안

- OAuth 토큰은 **로컬에서만 읽고**, **오직** Anthropic 자체 엔드포인트로만 HTTPS(rustls)로
  전송됩니다. 제3자로는 절대 전송되지 않습니다.
- 토큰은 **로그에 남기지 않고**, 프론트엔드로 전달하지 않으며, 에러 메시지에도 포함하지
  않습니다. 백엔드에는 자격 증명 관련 로깅이 전혀 없습니다.
- 갱신된 토큰은 **메모리에만** 보관하며 기본적으로 `~/.claude/.credentials.json` 에
  되쓰지 않습니다(Claude Code 자체 토큰 관리와 충돌하지 않도록).
- 저장소에는 **비밀 값이 전혀 없습니다.** 앱이 다루는 Claude 데이터는 실행 중 로컬
  `~/.claude` 디렉터리에만 존재합니다.

---

## ⚠️ 라이브 API 관련 주의

정확한 잔여 한도 수치는 로컬 OAuth 토큰을 사용하는 **비공식** 엔드포인트
(`GET /api/oauth/usage`)에서 가져옵니다 — `/usage` 가 보여주는 것과 동일한 데이터입니다.
다만:

- **공식 지원 API가 아니며**, 언제든 변경되거나 동작하지 않을 수 있습니다.
- Anthropic 약관은 OAuth 토큰 재사용을 제한하므로 본인 판단 하에 사용하세요.
- 해당 엔드포인트는 강한 레이트 리밋이 걸려 있어, 앱은 **최소 60초에 한 번**만 폴링하고
  오류 시 백오프합니다.

API가 비활성화되거나 사용 불가일 때는 트랜스크립트 기반 **로컬 추정**으로 폴백하며,
이는 항상 오프라인에서도 동작합니다. 설정에서 API를 완전히 끌 수도 있습니다.

---

## 🧪 개발

```bash
cargo test --manifest-path src-tauri/Cargo.toml   # 단위 테스트 60개 (분석·파싱·보안)
npm run build                                      # 타입 체크 + 프론트엔드 번들
```

구조: 순수 분석 로직은 `src-tauri/src/analytics/` 와 `src-tauri/src/jsonl/` 에 있습니다
(I/O 없음, `now` 를 주입받아 결정적이며 전부 단위 테스트됨). Tauri 레이어
(`commands`, `state`, `refresher`, `watcher`)가 이를 오케스트레이션하고 프론트엔드로
`usage://update` 이벤트를 푸시합니다.

**컨트리뷰터 가이드:** 아키텍처 맵·컨벤션·변경 레시피는 [`CLAUDE.md`](CLAUDE.md) 를
참고하세요. (Claude Code 사용자는 로컬 `.claude/skills/` 의 프로젝트 워크플로 스킬 —
`ctm-dev`, `ctm-add-metric`, `ctm-release` — 을 활용할 수 있습니다. 저장소에는 게시되지 않습니다.)

---

## 📄 라이선스

[MIT](LICENSE) © 2026 no1fc. Anthropic과 무관한 비공식 커뮤니티 도구입니다.
