#!/bin/sh
# Memory of the machine, one line every 2 s, from inside the container:
#   epoch  server_rss  postgres_pss  cgroup_anon  cgroup_file  cgroup_current  oom_kills  shmem   (MiB)
# Postgres is summed as PSS, not RSS: its backends share shared_buffers, and
# adding up RSS counts that memory once per process.
while :; do
    server=$(cat /run/hoard/server.pid 2>/dev/null)
    srss=$(awk '/^VmRSS/{print int($2/1024)}' "/proc/$server/status" 2>/dev/null)
    ppss=0
    for p in $(pgrep -u postgres); do
        # smaps_rollup needs ptrace access; the owner has it, root in a container does not.
        kb=$(setpriv --reuid=postgres --regid=postgres --clear-groups cat "/proc/$p/smaps_rollup" 2>/dev/null | awk '/^Pss:/{print $2}')
        ppss=$((ppss + ${kb:-0}))
    done
    awk -v t="$(date +%s)" -v s="${srss:-0}" -v p="$((ppss / 1024))" \
        -v cur="$(cat /sys/fs/cgroup/memory.current)" \
        -v oom="$(awk '/^oom_kill /{print $2}' /sys/fs/cgroup/memory.events)" \
        '/^anon /{a=$2} /^file /{f=$2} /^shmem /{m=$2} END{printf "%s %d %d %d %d %d %s %d\n", t, s, p, a/1048576, f/1048576, cur/1048576, oom, m/1048576}' \
        /sys/fs/cgroup/memory.stat
    sleep 2
done
