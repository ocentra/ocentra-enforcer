FROM python:3.12-slim@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
RUN apt-get update \
 && apt-get install --no-install-recommends -y ca-certificates \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY app.py /app/app.py
USER 10001
ENTRYPOINT ["python", "/app/app.py"]
