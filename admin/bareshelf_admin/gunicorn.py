import os

for k, v in os.environ.items():
    if k.startswith("GUNICORN_"):
        key = k.split("_", 1)[1].lower()
        locals()[key] = v

errorlog = '-'
loglevel = 'info'

accesslog = '-'
access_log_format = '%(t)s "%(r)s" %(s)s %(b)s "%(f)s" "%(a)s" %(L)s'
