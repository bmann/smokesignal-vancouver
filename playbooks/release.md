# Release Playbook

To release a version of Smoke Signal:

1. Set the version in `Cargo.toml`
2. Set the version in `Dockerfile`
3. Commit the changes `git commit -m "release: X.Y.Z"`
4. Tag the commit `git tag -s -m "vX.Y.Z" X.Y.Z`
5. Build the container `docker build -t repository/smokesignal:latest .`
