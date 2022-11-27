#!/bin/bash

docker build -t gcr.io/gamesite-369621/backend:latest .
docker push gcr.io/gamesite-369621/backend:latest