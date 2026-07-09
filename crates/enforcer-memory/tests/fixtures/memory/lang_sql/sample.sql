CREATE FUNCTION add_one(x integer) RETURNS integer AS $$
BEGIN
  RETURN x + 1;
END;
$$ LANGUAGE plpgsql;

CREATE TYPE mood AS ENUM ('sad', 'ok', 'happy');

SELECT add_one(1), upper('x') FROM accounts WHERE CASE WHEN id > 1 THEN true ELSE false END;
