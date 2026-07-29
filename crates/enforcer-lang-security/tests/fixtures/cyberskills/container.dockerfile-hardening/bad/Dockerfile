FROM python:latest
ENV API_KEY=sk-live-abc123def456
RUN curl https://example.com/install.sh | bash
RUN apt-get install -y curl
ADD https://example.com/app.tar.gz /app/
COPY app.py /app/app.py
CMD ["python", "/app/app.py"]
