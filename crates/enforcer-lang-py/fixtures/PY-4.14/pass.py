import logging

logger = logging.getLogger(__name__)


def load(value: str) -> None:
    logger.info("loading %s", value)
