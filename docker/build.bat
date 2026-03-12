@echo off

docker build --no-cache -t power2all/torrust-actix:v4.2.3 -t power2all/torrust-actix:latest .
docker push power2all/torrust-actix:v4.2.3
docker push power2all/torrust-actix:latest