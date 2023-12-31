#!/bin/bash
set -e

IMAGE_NAME=fritz-log-parser
REGISTRY=registry.debian.home.arpa

docker build -t $IMAGE_NAME .
docker tag $IMAGE_NAME $REGISTRY/$IMAGE_NAME
docker push $REGISTRY/$IMAGE_NAME
