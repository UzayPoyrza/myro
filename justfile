default:
    cargo run -p myro-tui

build:
    cargo build --workspace

check:
    cargo check --workspace

clippy:
    cargo clippy --workspace

test:
    cargo test --workspace

fmt:
    cargo fmt

predict *args:
    cargo run -p myro-predict -- {{args}}

tag-skills handle *args:
    cargo run -p myro-predict -- tag-skills --handle {{handle}} {{args}}

mock-tui:
    MYRO_EPHEMERAL=1 cargo run -p myro-tui

bench *args:
    cargo run -p myro-bench -- run {{args}}

bench-report:
    cargo run -p myro-bench -- report

# Web Trainer (apps/web)
web-dev:
    cd apps/web && npm run dev

web-build:
    cd apps/web && npm run build

web-setup:
    cd apps/web && npm install && npm run setup

web-seed:
    cd apps/web && npm run db:seed

web-reset:
    cd apps/web && npm run db:reset

# Teaser page (apps/teaser)
teaser-dev:
    cd apps/teaser && python3 -m http.server 8080

teaser-deploy:
    npx wrangler pages deploy apps/teaser/ --project-name myro-teaser
