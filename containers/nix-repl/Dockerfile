FROM debian:bullseye-slim

RUN apt-get update && \
    apt-get install -y curl ca-certificates xz-utils git build-essential passwd && \
    rm -rf /var/lib/apt/lists/*

# Proper nixbld group + users
RUN groupadd -g 30000 nixbld && \
    for i in $(seq 1 10); do \
      useradd -m -s /bin/bash -u $((30000 + i)) -g nixbld nixbld$i; \
    done

# Install Nix 2.32.4
RUN mkdir -m 0755 /nix && \
    curl -L https://releases.nixos.org/nix/nix-2.32.4/nix-2.32.4-x86_64-linux.tar.xz \
      | tar -xJf - -C /tmp && \
    /tmp/nix-2.32.4-x86_64-linux/install --no-daemon --no-modify-profile

ENV NIX_CONFIG="experimental-features = nix-command flakes"
ENV PATH=/root/.nix-profile/bin:/root/.cargo/bin:$PATH
