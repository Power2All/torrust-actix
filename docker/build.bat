@echo off

docker build --no-cache -f "%~dp0Dockerfile" -t power2all/torrust-actix:v4.2.24 -t power2all/torrust-actix:latest "%~dp0.."
docker push power2all/torrust-actix:v4.2.24
docker push power2all/torrust-actix:latest