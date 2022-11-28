#!/bin/bash

set -e

# Build and push image with newest code
docker build -t gcr.io/gamesite-369621/backend:latest .
docker push gcr.io/gamesite-369621/backend:latest

# Restart the backend to pull the new latest image
instance_name=$(gcloud compute instances list --format=json | jq -r '.[0].name')
gcloud compute instances reset "${instance_name}"