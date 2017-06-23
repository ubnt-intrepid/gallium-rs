#!/bin/bash
set -euo pipefail

curl -X POST "http://localhost:3000/api/v1/users" -d '
{
  "username": "user1",
  "email_address": "user1@example.com",
  "password": "user1"
}
'
echo

curl -X POST "http://localhost:3000/api/v1/projects" -d '
{
  "user": "user1",
  "project": "project1",
  "description": "Awesome project"
}
'
echo
