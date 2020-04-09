FROM python:3.8-slim

WORKDIR /opt/app

RUN apt-get update \
      && apt-get install -y --no-install-recommends \
      libpq-dev \
      build-essential \
      && rm -rf /var/lib/apt/lists/*

# Python dependencies
RUN pip3 install -U pip poetry
COPY pyproject.toml poetry.lock ./
ARG POETRY_FLAGS="--no-dev"
RUN poetry config virtualenvs.create false && poetry install ${POETRY_FLAGS}

COPY . .

USER www-data

ENV FLASK_APP=bareshelf_admin.application

CMD ["./run.sh"]
