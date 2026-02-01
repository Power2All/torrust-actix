@echo off

docker build -t power2all/torrust-actix:v4.1.0 -t power2all/torrust-actix:latest .
docker push power2all/torrust-actix:v4.1.0
docker push power2all/torrust-actix:latest