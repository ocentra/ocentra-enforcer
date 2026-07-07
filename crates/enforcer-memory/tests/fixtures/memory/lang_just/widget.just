import 'common.just'

build: setup
    cargo build

setup:
    echo "setting up"

test arg="default":
    cargo test {{arg}}
