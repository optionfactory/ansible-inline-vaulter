#!/bin/bash

file_name=ansible-inline-vault-viewer
project_name=ansible-inline-vault-viewer
token=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

download_url=$(curl -L \
  -H "Accept: application/vnd.github+json" \
  -H "Authorization: Bearer $token" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  https://api.github.com/repos/optionfactory/$project_name/actions/artifacts | jq '.artifacts[]|.name,.archive_download_url' | grep -m 1 -A 1 $file_name | tail -1 | tr -d \")

curl -L --output $file_name \
  -H "Accept: application/vnd.github+json" \
  -H "Authorization: Bearer $token" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  $download_url  

unzip -o $file_name
chmod +x $file_name