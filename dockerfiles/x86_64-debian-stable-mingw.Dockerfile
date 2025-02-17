FROM debian:stable

ENV PATH="/root/.cargo/bin:${PATH}"
RUN apt update -y && apt install -y gcc curl gcc-mingw-w64-x86-64
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | bash -s -- -y
RUN echo "[target.x86_64-pc-windows-gnu]" >  /root/.cargo/config
RUN echo "linker = \"/usr/bin/x86_64-w64-mingw32-gcc\"" >>  /root/.cargo/config
RUN echo "ar = \"/usr/bin/x86_64-w64-mingw32-ar\"" >>  /root/.cargo/config
RUN rustup target add x86_64-pc-windows-gnu








