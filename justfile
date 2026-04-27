default:
    @cd "{{justfile_directory()}}" && just --list

check:
    @bash "{{justfile_directory()}}/just/check.sh"

build:
    @bash "{{justfile_directory()}}/just/build.sh"

run:
    @bash "{{justfile_directory()}}/just/run.sh"

deploy:
    @bash "{{justfile_directory()}}/just/deploy-macos.sh"

restart:
    @bash "{{justfile_directory()}}/just/restart-macos.sh"

status:
    @bash "{{justfile_directory()}}/just/status-macos.sh"

logs lines="120":
    @bash "{{justfile_directory()}}/just/logs-macos.sh" "{{lines}}"

undeploy:
    @bash "{{justfile_directory()}}/just/undeploy-macos.sh"
