# -----------------------------------------------------------------------------------------

FROM alpine as server-build

# Install server build dependencies
RUN apk add git cargo

# Compile the server
WORKDIR /home
RUN git clone https://github.com/Gabie-of-the-Bo/picorun-runner.git
WORKDIR /home/picorun-runner
RUN cargo build --release

# -----------------------------------------------------------------------------------------

FROM alpine as python-build

# Prepare python execution environment
RUN apk add python3 py3-pip make automake gcc g++ subversion python3-dev

RUN python3 -m venv /opt/python-env
ENV PATH="/opt/python-env/bin:$PATH"

RUN python -m pip install numpy pandas
RUN pip uninstall -y pip

# -----------------------------------------------------------------------------------------

FROM alpine as final

RUN apk add libgcc libstdc++ python3

COPY --from=server-build /home/picorun-runner/target/release/server /home/picorun-runner/target/release/
COPY --from=server-build /home/picorun-runner/environments/ /home/picorun-runner/environments/

COPY --from=python-build /opt/python-env /opt/python-env
ENV PATH="/opt/python-env/bin:$PATH"