FROM debian:stable-slim

RUN apt-get update && \
    apt-get install -y curl ca-certificates python3 xz-utils && \
    rm -rf /var/lib/apt/lists/*

# Create the user and the /nix directory as root
RUN useradd -m nixuser
RUN mkdir -m 0755 /nix && chown nixuser /nix

USER nixuser
ENV USER=nixuser
ENV NIX_INSTALLER_NO_MODIFY_PROFILE=1

RUN curl -L https://nixos.org/nix/install -o /tmp/install-nix.sh \
 && sh /tmp/install-nix.sh --no-daemon \
 && rm /tmp/install-nix.sh

# Enable nix-command and flakes for this user
RUN mkdir -p /home/nixuser/.config/nix && \
    echo 'experimental-features = nix-command flakes' > /home/nixuser/.config/nix/nix.conf

ENV NIX_PATH=/home/nixuser/.nix-profile/etc/nix
ENV PATH=/home/nixuser/.nix-profile/bin:$PATH

WORKDIR /app
COPY --chown=nixuser:nixuser server.py /app/server.py

EXPOSE 8080
CMD ["python3", "/app/server.py"]
