FROM gitpod/workspace-full:2022-09-03-08-31-56

USER gitpod

ENV VOLTA_HOME=/workspace/.volta

ENV PATH="$VOLTA_HOME/bin:$PATH:$HOME/emsdk:$HOME/emsdk/upstream/emscripten"

RUN git clone https://github.com/emscripten-core/emsdk.git && \
    emsdk install 3.1.20 && \
    emsdk activate 3.1.20

# Install custom tools, runtime, etc. using apt-get
# For example, the command below would install "bastet" - a command line tetris clone:
#
# RUN sudo apt-get -q update && \
#     sudo apt-get install -yq bastet && \
#     sudo rm -rf /var/lib/apt/lists/*
#
# More information: https://www.gitpod.io/docs/config-docker/
