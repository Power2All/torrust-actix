@echo off

docker build --no-cache -f "%~dp0Dockerfile" -t power2all/torrust-actix:v4.2.25 -t power2all/torrust-actix:latest "%~dp0.."
docker push power2all/torrust-actix:v4.2.25
docker push power2all/torrust-actix:latest