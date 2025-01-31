#!/usr/bin/env bash
set -e

pushd .

# The following line ensure we run from the project root
PROJECT_ROOT=`git rev-parse --show-toplevel`
cd $PROJECT_ROOT

# Find the current version from Cargo.toml
VERSION=`grep "^version" ./standalone/node/Cargo.toml | egrep -o "([0-9\.]+)"`
GITUSER=webb-tools
IMAGE_NAME=protocol-substrate-standalone-node

# Build the image
echo "Building ${ gitUSER}/${IMAGE_NAME}:latest docker image, hang on!"
time docker build -f ./docker/Standalone.Dockerfile -t ${ gitUSER}/${IMAGE_NAME}:latest .
docker tag ${ gitUSER}/${IMAGE_NAME}:latest ${ gitUSER}/${IMAGE_NAME}:v${ version}

# Show the list of available images for this repo
echo "Image is ready"
docker images | grep ${IMAGE_NAME}

popd
