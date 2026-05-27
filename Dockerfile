FROM scratch

COPY createrepo_rs /usr/local/bin/createrepo_rs

ENTRYPOINT ["createrepo_rs"]
