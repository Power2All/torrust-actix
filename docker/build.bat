@echo off

docker build -t power2all/torrust-actix:v4.0.15 -t power2all/torrust-actix:latest .
docker push power2all/torrust-actix:v4.0.15
docker push power2all/torrust-actix:latest