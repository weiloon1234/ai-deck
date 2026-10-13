# The preparation tool supplies the digest from model-catalog/catalog.json.
ARG BASE_IMAGE
FROM ${BASE_IMAGE}

COPY launcher.py /opt/ai-deck/launcher.py
ENTRYPOINT ["python3", "-u", "/opt/ai-deck/launcher.py"]
CMD []
