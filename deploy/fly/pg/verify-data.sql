-- Row count and an order-independent checksum for every table in the cloud
-- schemas, one line per table (`psql -At -f`). Run it against the source and
-- the restored copy and diff the two outputs: any difference means the copy is
-- not the source.
--
-- The checksum hashes each row's text form, so every setting that changes how
-- a value prints is pinned here. Without that, two identical databases with a
-- different TimeZone or DateStyle would disagree on every timestamp.

\set ON_ERROR_STOP on
\set QUIET on
\pset footer off

SET TimeZone = 'UTC';
SET DateStyle = 'ISO, MDY';
SET IntervalStyle = 'postgres';
SET extra_float_digits = 1;
SET bytea_output = 'hex';

SELECT format(
           $q$SELECT %L, count(*), coalesce(sum(('x' || left(md5(t::text), 15))::bit(60)::bigint), 0) FROM %I.%I t$q$,
           n.nspname || '.' || c.relname, n.nspname, c.relname)
  FROM pg_class c
  JOIN pg_namespace n ON n.oid = c.relnamespace
 WHERE n.nspname IN ('public', 'ops')
   AND c.relkind IN ('r', 'p')
 ORDER BY n.nspname, c.relname
\gexec
