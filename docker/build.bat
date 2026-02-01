@echo off

docker build -t power2all/torrust-actix:v4.0.17 -t power2all/torrust-actix:latest .
docker push power2all/torrust-actix:v4.0.17
docker push power2all/torrust-actix:latest