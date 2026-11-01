#!/bin/bash
set -e

npx @redocly/openapi-cli@1.0.0-beta.95 bundle openapi.yaml > openapi-bundled.yaml


npx @redocly/openapi-cli@1.0.0-beta.95 bundle openapi-bundled.yaml --ext yaml -d > openapi-deref.yaml
npx @redocly/openapi-cli@1.0.0-beta.95 bundle openapi-bundled.yaml --ext json > openapi-deref.json


rm openapi-bundled.yaml
