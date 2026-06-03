-- DAG provenance: each snapshot records the version it descends from.
-- NULL = root (first version of the save). This turns the linear
-- version_num log into a graph so the server can detect a non-fast-forward
-- (divergent) push from a second device instead of silently appending it
-- as the next version and letting last-writer-win clobber the other line.
ALTER TABLE snapshots ADD COLUMN parent_version INTEGER;
