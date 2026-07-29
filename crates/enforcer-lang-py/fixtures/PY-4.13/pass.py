try:
    load()
except ValueError as error:
    log_error(error)
