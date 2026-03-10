@echo off

docker build --no-cache -t power2all/torrust-actix:v4.2.1 -t power2all/torrust-actix:latest .
docker push power2all/torrust-actix:v4.2.1
docker push power2all/torrust-actix:latest