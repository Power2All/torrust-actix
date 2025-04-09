#!/bin/sh

if [ ! -f "config.toml" ]
then
  ./torrust-actix --create-config
fi

./torrust-actix