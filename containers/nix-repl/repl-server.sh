#!/usr/bin/env bash
set -eu

PORT=8080

handle_request() {
  # Read the entire HTTP request into a temp file
  req="$(mktemp)"
  cat >"$req"

  # First line: method path version
  read -r method path version <"$req"

  # If it's CORS preflight, respond and exit early
  if [ "$method" = "OPTIONS" ]; then
    printf 'HTTP/1.1 204 No Content\r\n'
    printf 'Access-Control-Allow-Origin: *\r\n'
    printf 'Access-Control-Allow-Methods: POST, OPTIONS\r\n'
    printf 'Access-Control-Allow-Headers: Content-Type\r\n'
    printf 'Access-Control-Max-Age: 86400\r\n'
    printf '\r\n'
    rm -f "$req"
    return
  fi

  # Extract body (everything after the blank line)
  body="$(awk 'f{print} /^$/{f=1}' "$req")"
  rm -f "$req"

  code="$(printf '%s' "$body" | jq -r '.code // ""')"

  if [ -z "$code" ]; then
    resp='{"error":"No code provided"}'
    printf 'HTTP/1.1 400 Bad Request\r\n'
    printf 'Content-Type: application/json\r\n'
    printf 'Access-Control-Allow-Origin: *\r\n'
    printf 'Content-Length: %s\r\n\r\n' "$(printf '%s' "$resp" | wc -c)"
    printf '%s' "$resp"
    return
  fi

  tmpdir="$(mktemp -d)"
  stdout_file="$tmpdir/stdout"
  stderr_file="$tmpdir/stderr"

  if nix eval --raw --expr "$code" >"$stdout_file" 2>"$stderr_file"; then
    stdout="$(cat "$stdout_file")"
    resp="$(jq -n --arg out "$stdout" '{stdout: $out}')"
  else
    stderr="$(cat "$stderr_file")"
    resp="$(jq -n --arg err "$stderr" '{error: $err}')"
  fi

  rm -rf "$tmpdir"

  printf 'HTTP/1.1 200 OK\r\n'
  printf 'Content-Type: application/json\r\n'
  printf 'Access-Control-Allow-Origin: *\r\n'
  printf 'Content-Length: %s\r\n\r\n' "$(printf '%s' "$resp" | wc -c)"
  printf '%s' "$resp"
}

export -f handle_request

# socat will call handle_request for each connection
while true; do
  socat -v TCP-LISTEN:$PORT,reuseaddr,fork EXEC:"bash -c handle_request"
done
