[English](README.md) | [한국어](README.ko.md) | **日本語** | [简体中文](README.zh-CN.md)

# bmo-artifact

ドキュメントを保管、検索、共有する、独自の API を持つ単一のサービスです。記録、学習ブック、取り込んだ Web ページと
HTML ページを扱い、各ドキュメントはファイル、参照するページ、共有リンク、そして閲覧用に共有された読み手の会社の
メンバーを持ちます。ハーネスは MCP サーバー `bmo-artifact mcp` と `bmo-artifact` スキルを通じて、その他の
呼び出し元は HTTP API と Rust クライアントを通じてアクセスします。Web ページは bmo-explorer のクライアントで
読み込み、他のユニットは呼び出しません。

## 構成

| フォルダ | 内容 |
|---|---|
| `src/` | ライブラリと唯一のバイナリ `bmo-artifact`: ルート、ストア、クライアント、MCP サーバー、コマンド |
| `lang/` | ドキュメント文言ファイルの英語の既定値 |
| `skills/` | `bmo-artifact` スキル |
| `client/` | コンソールバンドル |
| `tests/` | リポジトリのチェックを含むテスト |
| `deploy/` | 中立的なサービスユニットのテンプレート |
| `docs/` | サービスの動作を定める正典ドキュメント |

ルートには Cargo マニフェスト、`VERSION`、`build.rs`、`testbed.toml`、`install.toml`、`rustfmt.toml`、`CLAUDE.md`、
`PERSONAL-DATA.md`、`LICENSE`、`settings.toml`、そしてデプロイ時に与えるローカル設定の例である
`settings.local.toml.example` があります。

## ドキュメント

| ドキュメント | 内容 |
|---|---|
| `docs/api.md` | API: ルート、イベント、コマンド、クライアント、MCP ツール、bmo-explorer の呼び出し |
| `docs/openapi.json` | コードから生成した OpenAPI 3.1 ドキュメントとしての API |
| `docs/artifact.md` | ドキュメントの設計。API がそれを扱い、構成がそれを配置する |
| `docs/layout.md` | 構成、配管、コンソールバンドル、ストレージ、バージョン、宣言、チェック |
| `PERSONAL-DATA.md` | どのファイルにも置いてはならない個人データと、それを守るチェック |

## バージョン

- **リポジトリ全体でバージョンは一つで、** 最初のリリースまでは `0.1.0`、その後は `0.1.0-rev.<n>` です。
  `VERSION` がそれを持ち、クレートとコンソールバンドルのパッケージも同じ値で、ヘルスルート、OpenAPI ドキュメント、
  MCP サーバーはこのバージョンを返します。

## コマンド

```sh
cargo check --all-targets
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run --bin bmo-artifact -- serve
cargo run --bin bmo-artifact -- mcp
cargo run --bin bmo-artifact -- openapi
```
