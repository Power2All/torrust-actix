#!/bin/sh

cd /app/torrust-actix/target/release

if [ ! -f "config.toml" ]
then
  ./torrust-actix --create-config
fi

./torrust-actix