#!/usr/bin/env bash

flask db upgrade
exec gunicorn --config bareshelf_admin/gunicorn.py -b ":6001" bareshelf_admin.entrypoint:app
