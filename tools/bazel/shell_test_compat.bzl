def _shell_quote(value):
    return "'" + value.replace("'", "'\\''") + "'"

def _transition_shell_test_impl(ctx):
    if len(ctx.files.srcs) != 1:
        fail("shell script test requires exactly one script in srcs")

    script = ctx.files.srcs[0]
    executable = ctx.actions.declare_file(ctx.label.name + ".test.sh")
    args = " ".join([_shell_quote(arg) for arg in ctx.attr.script_args])
    content = """#!/usr/bin/env bash
set -euo pipefail

script_path="${{TEST_SRCDIR}}/${{TEST_WORKSPACE}}/{script_short_path}"
if [ ! -f "$script_path" ]; then
  script_path="{script_exec_path}"
fi

exec bash "$script_path" {args}
""".format(
        args = args,
        script_exec_path = script.path,
        script_short_path = script.short_path,
    )
    ctx.actions.write(executable, content, is_executable = True)
    data_files = []
    for target in ctx.attr.data:
        data_files.extend(target[DefaultInfo].files.to_list())

    return [DefaultInfo(
        executable = executable,
        runfiles = ctx.runfiles(files = [script] + data_files),
    )]

shell_script_test = rule(
    implementation = _transition_shell_test_impl,
    attrs = {
        "script_args": attr.string_list(),
        "data": attr.label_list(allow_files = True),
        "srcs": attr.label_list(allow_files = True, mandatory = True),
    },
    test = True,
)

transition_shell_test = shell_script_test
