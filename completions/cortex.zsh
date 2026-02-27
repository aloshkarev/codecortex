#compdef cortex

autoload -U is-at-least

_cortex() {
    typeset -A opt_args
    typeset -a _arguments_options
    local ret=1

    if is-at-least 5.2; then
        _arguments_options=(-s -S -C)
    else
        _arguments_options=(-s -C)
    fi

    local context curcontext="$curcontext" state line
    _arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
":: :_cortex_commands" \
"*::: :->cortex" \
&& ret=0
    case $state in
    (cortex)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cortex-command-$line[1]:"
        case $line[1] in
            (setup)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(doctor)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(mcp)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_cortex__mcp_commands" \
"*::: :->mcp" \
&& ret=0

    case $state in
    (mcp)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cortex-mcp-command-$line[1]:"
        case $line[1] in
            (start)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(tools)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_cortex__mcp__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cortex-mcp-help-command-$line[1]:"
        case $line[1] in
            (start)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(tools)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(index)
_arguments "${_arguments_options[@]}" : \
'--force[]' \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
':path:_default' \
&& ret=0
;;
(watch)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
':path:_default' \
&& ret=0
;;
(unwatch)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
':path:_default' \
&& ret=0
;;
(find)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_cortex__find_commands" \
"*::: :->find" \
&& ret=0

    case $state in
    (find)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cortex-find-command-$line[1]:"
        case $line[1] in
            (name)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
':name:_default' \
&& ret=0
;;
(pattern)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
':pattern:_default' \
&& ret=0
;;
(type)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
':kind:_default' \
&& ret=0
;;
(content)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
':query:_default' \
&& ret=0
;;
(decorator)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
':name:_default' \
&& ret=0
;;
(argument)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
':name:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_cortex__find__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cortex-find-help-command-$line[1]:"
        case $line[1] in
            (name)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(pattern)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(type)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(content)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(decorator)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(argument)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(analyze)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_cortex__analyze_commands" \
"*::: :->analyze" \
&& ret=0

    case $state in
    (analyze)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cortex-analyze-command-$line[1]:"
        case $line[1] in
            (callers)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
':target:_default' \
&& ret=0
;;
(callees)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
':target:_default' \
&& ret=0
;;
(chain)
_arguments "${_arguments_options[@]}" : \
'--depth=[]:DEPTH:_default' \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
':from:_default' \
':to:_default' \
&& ret=0
;;
(hierarchy)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
':class:_default' \
&& ret=0
;;
(deps)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
':module:_default' \
&& ret=0
;;
(dead-code)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(complexity)
_arguments "${_arguments_options[@]}" : \
'--top=[]:TOP:_default' \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(overrides)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
':method:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_cortex__analyze__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cortex-analyze-help-command-$line[1]:"
        case $line[1] in
            (callers)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(callees)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(chain)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(hierarchy)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(deps)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(dead-code)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(complexity)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(overrides)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(bundle)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_cortex__bundle_commands" \
"*::: :->bundle" \
&& ret=0

    case $state in
    (bundle)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cortex-bundle-command-$line[1]:"
        case $line[1] in
            (export)
_arguments "${_arguments_options[@]}" : \
'--repo=[]:REPO:_files' \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
':output:_files' \
&& ret=0
;;
(import)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
':path:_files' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_cortex__bundle__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cortex-bundle-help-command-$line[1]:"
        case $line[1] in
            (export)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(import)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(config)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_cortex__config_commands" \
"*::: :->config" \
&& ret=0

    case $state in
    (config)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cortex-config-command-$line[1]:"
        case $line[1] in
            (show)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(set)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
':key:_default' \
':value:_default' \
&& ret=0
;;
(reset)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_cortex__config__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cortex-config-help-command-$line[1]:"
        case $line[1] in
            (show)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(set)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(reset)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(clean)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(delete)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
':path:_default' \
&& ret=0
;;
(stats)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(query)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
':cypher:_default' \
&& ret=0
;;
(jobs)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_cortex__jobs_commands" \
"*::: :->jobs" \
&& ret=0

    case $state in
    (jobs)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cortex-jobs-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
':id:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_cortex__jobs__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cortex-jobs-help-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(debug)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_cortex__debug_commands" \
"*::: :->debug" \
&& ret=0

    case $state in
    (debug)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cortex-debug-command-$line[1]:"
        case $line[1] in
            (capsule)
_arguments "${_arguments_options[@]}" : \
'--max-items=[Maximum items in capsule]:MAX_ITEMS:_default' \
'--explain[Explain the capsule building process]' \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
':symbol -- Symbol name to build capsule for:_default' \
&& ret=0
;;
(cache)
_arguments "${_arguments_options[@]}" : \
'--clear[Clear the cache]' \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(trace)
_arguments "${_arguments_options[@]}" : \
'-v[Enable verbose output]' \
'--verbose[Enable verbose output]' \
'--json[]' \
'-h[Print help]' \
'--help[Print help]' \
':query -- Query to trace:_default' \
&& ret=0
;;
(validate)
_arguments "${_arguments_options[@]}" : \
'--repo=[Repository path to validate]:REPO:_default' \
'--fix[Fix issues automatically]' \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_cortex__debug__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cortex-debug-help-command-$line[1]:"
        case $line[1] in
            (capsule)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(cache)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(trace)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(validate)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(completion)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
':shell -- Shell type (bash, zsh, fish, elvish, powershell):(bash elvish fish powershell zsh)' \
&& ret=0
;;
(interactive)
_arguments "${_arguments_options[@]}" : \
'--json[]' \
'*-v[]' \
'*--verbose[]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_cortex__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cortex-help-command-$line[1]:"
        case $line[1] in
            (setup)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(doctor)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(mcp)
_arguments "${_arguments_options[@]}" : \
":: :_cortex__help__mcp_commands" \
"*::: :->mcp" \
&& ret=0

    case $state in
    (mcp)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cortex-help-mcp-command-$line[1]:"
        case $line[1] in
            (start)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(tools)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(index)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(watch)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(unwatch)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(find)
_arguments "${_arguments_options[@]}" : \
":: :_cortex__help__find_commands" \
"*::: :->find" \
&& ret=0

    case $state in
    (find)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cortex-help-find-command-$line[1]:"
        case $line[1] in
            (name)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(pattern)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(type)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(content)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(decorator)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(argument)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(analyze)
_arguments "${_arguments_options[@]}" : \
":: :_cortex__help__analyze_commands" \
"*::: :->analyze" \
&& ret=0

    case $state in
    (analyze)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cortex-help-analyze-command-$line[1]:"
        case $line[1] in
            (callers)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(callees)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(chain)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(hierarchy)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(deps)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(dead-code)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(complexity)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(overrides)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(bundle)
_arguments "${_arguments_options[@]}" : \
":: :_cortex__help__bundle_commands" \
"*::: :->bundle" \
&& ret=0

    case $state in
    (bundle)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cortex-help-bundle-command-$line[1]:"
        case $line[1] in
            (export)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(import)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(config)
_arguments "${_arguments_options[@]}" : \
":: :_cortex__help__config_commands" \
"*::: :->config" \
&& ret=0

    case $state in
    (config)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cortex-help-config-command-$line[1]:"
        case $line[1] in
            (show)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(set)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(reset)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(clean)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(delete)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(stats)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(query)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(jobs)
_arguments "${_arguments_options[@]}" : \
":: :_cortex__help__jobs_commands" \
"*::: :->jobs" \
&& ret=0

    case $state in
    (jobs)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cortex-help-jobs-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(debug)
_arguments "${_arguments_options[@]}" : \
":: :_cortex__help__debug_commands" \
"*::: :->debug" \
&& ret=0

    case $state in
    (debug)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:cortex-help-debug-command-$line[1]:"
        case $line[1] in
            (capsule)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(cache)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(trace)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(validate)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(completion)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(interactive)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
}

(( $+functions[_cortex_commands] )) ||
_cortex_commands() {
    local commands; commands=(
'setup:' \
'doctor:' \
'mcp:' \
'index:' \
'watch:' \
'unwatch:' \
'find:' \
'analyze:' \
'bundle:' \
'config:' \
'clean:' \
'list:' \
'delete:' \
'stats:' \
'query:' \
'jobs:' \
'debug:' \
'completion:Generate shell completion scripts' \
'interactive:Start interactive REPL mode' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cortex commands' commands "$@"
}
(( $+functions[_cortex__analyze_commands] )) ||
_cortex__analyze_commands() {
    local commands; commands=(
'callers:' \
'callees:' \
'chain:' \
'hierarchy:' \
'deps:' \
'dead-code:' \
'complexity:' \
'overrides:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cortex analyze commands' commands "$@"
}
(( $+functions[_cortex__analyze__callees_commands] )) ||
_cortex__analyze__callees_commands() {
    local commands; commands=()
    _describe -t commands 'cortex analyze callees commands' commands "$@"
}
(( $+functions[_cortex__analyze__callers_commands] )) ||
_cortex__analyze__callers_commands() {
    local commands; commands=()
    _describe -t commands 'cortex analyze callers commands' commands "$@"
}
(( $+functions[_cortex__analyze__chain_commands] )) ||
_cortex__analyze__chain_commands() {
    local commands; commands=()
    _describe -t commands 'cortex analyze chain commands' commands "$@"
}
(( $+functions[_cortex__analyze__complexity_commands] )) ||
_cortex__analyze__complexity_commands() {
    local commands; commands=()
    _describe -t commands 'cortex analyze complexity commands' commands "$@"
}
(( $+functions[_cortex__analyze__dead-code_commands] )) ||
_cortex__analyze__dead-code_commands() {
    local commands; commands=()
    _describe -t commands 'cortex analyze dead-code commands' commands "$@"
}
(( $+functions[_cortex__analyze__deps_commands] )) ||
_cortex__analyze__deps_commands() {
    local commands; commands=()
    _describe -t commands 'cortex analyze deps commands' commands "$@"
}
(( $+functions[_cortex__analyze__help_commands] )) ||
_cortex__analyze__help_commands() {
    local commands; commands=(
'callers:' \
'callees:' \
'chain:' \
'hierarchy:' \
'deps:' \
'dead-code:' \
'complexity:' \
'overrides:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cortex analyze help commands' commands "$@"
}
(( $+functions[_cortex__analyze__help__callees_commands] )) ||
_cortex__analyze__help__callees_commands() {
    local commands; commands=()
    _describe -t commands 'cortex analyze help callees commands' commands "$@"
}
(( $+functions[_cortex__analyze__help__callers_commands] )) ||
_cortex__analyze__help__callers_commands() {
    local commands; commands=()
    _describe -t commands 'cortex analyze help callers commands' commands "$@"
}
(( $+functions[_cortex__analyze__help__chain_commands] )) ||
_cortex__analyze__help__chain_commands() {
    local commands; commands=()
    _describe -t commands 'cortex analyze help chain commands' commands "$@"
}
(( $+functions[_cortex__analyze__help__complexity_commands] )) ||
_cortex__analyze__help__complexity_commands() {
    local commands; commands=()
    _describe -t commands 'cortex analyze help complexity commands' commands "$@"
}
(( $+functions[_cortex__analyze__help__dead-code_commands] )) ||
_cortex__analyze__help__dead-code_commands() {
    local commands; commands=()
    _describe -t commands 'cortex analyze help dead-code commands' commands "$@"
}
(( $+functions[_cortex__analyze__help__deps_commands] )) ||
_cortex__analyze__help__deps_commands() {
    local commands; commands=()
    _describe -t commands 'cortex analyze help deps commands' commands "$@"
}
(( $+functions[_cortex__analyze__help__help_commands] )) ||
_cortex__analyze__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'cortex analyze help help commands' commands "$@"
}
(( $+functions[_cortex__analyze__help__hierarchy_commands] )) ||
_cortex__analyze__help__hierarchy_commands() {
    local commands; commands=()
    _describe -t commands 'cortex analyze help hierarchy commands' commands "$@"
}
(( $+functions[_cortex__analyze__help__overrides_commands] )) ||
_cortex__analyze__help__overrides_commands() {
    local commands; commands=()
    _describe -t commands 'cortex analyze help overrides commands' commands "$@"
}
(( $+functions[_cortex__analyze__hierarchy_commands] )) ||
_cortex__analyze__hierarchy_commands() {
    local commands; commands=()
    _describe -t commands 'cortex analyze hierarchy commands' commands "$@"
}
(( $+functions[_cortex__analyze__overrides_commands] )) ||
_cortex__analyze__overrides_commands() {
    local commands; commands=()
    _describe -t commands 'cortex analyze overrides commands' commands "$@"
}
(( $+functions[_cortex__bundle_commands] )) ||
_cortex__bundle_commands() {
    local commands; commands=(
'export:' \
'import:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cortex bundle commands' commands "$@"
}
(( $+functions[_cortex__bundle__export_commands] )) ||
_cortex__bundle__export_commands() {
    local commands; commands=()
    _describe -t commands 'cortex bundle export commands' commands "$@"
}
(( $+functions[_cortex__bundle__help_commands] )) ||
_cortex__bundle__help_commands() {
    local commands; commands=(
'export:' \
'import:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cortex bundle help commands' commands "$@"
}
(( $+functions[_cortex__bundle__help__export_commands] )) ||
_cortex__bundle__help__export_commands() {
    local commands; commands=()
    _describe -t commands 'cortex bundle help export commands' commands "$@"
}
(( $+functions[_cortex__bundle__help__help_commands] )) ||
_cortex__bundle__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'cortex bundle help help commands' commands "$@"
}
(( $+functions[_cortex__bundle__help__import_commands] )) ||
_cortex__bundle__help__import_commands() {
    local commands; commands=()
    _describe -t commands 'cortex bundle help import commands' commands "$@"
}
(( $+functions[_cortex__bundle__import_commands] )) ||
_cortex__bundle__import_commands() {
    local commands; commands=()
    _describe -t commands 'cortex bundle import commands' commands "$@"
}
(( $+functions[_cortex__clean_commands] )) ||
_cortex__clean_commands() {
    local commands; commands=()
    _describe -t commands 'cortex clean commands' commands "$@"
}
(( $+functions[_cortex__completion_commands] )) ||
_cortex__completion_commands() {
    local commands; commands=()
    _describe -t commands 'cortex completion commands' commands "$@"
}
(( $+functions[_cortex__config_commands] )) ||
_cortex__config_commands() {
    local commands; commands=(
'show:' \
'set:' \
'reset:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cortex config commands' commands "$@"
}
(( $+functions[_cortex__config__help_commands] )) ||
_cortex__config__help_commands() {
    local commands; commands=(
'show:' \
'set:' \
'reset:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cortex config help commands' commands "$@"
}
(( $+functions[_cortex__config__help__help_commands] )) ||
_cortex__config__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'cortex config help help commands' commands "$@"
}
(( $+functions[_cortex__config__help__reset_commands] )) ||
_cortex__config__help__reset_commands() {
    local commands; commands=()
    _describe -t commands 'cortex config help reset commands' commands "$@"
}
(( $+functions[_cortex__config__help__set_commands] )) ||
_cortex__config__help__set_commands() {
    local commands; commands=()
    _describe -t commands 'cortex config help set commands' commands "$@"
}
(( $+functions[_cortex__config__help__show_commands] )) ||
_cortex__config__help__show_commands() {
    local commands; commands=()
    _describe -t commands 'cortex config help show commands' commands "$@"
}
(( $+functions[_cortex__config__reset_commands] )) ||
_cortex__config__reset_commands() {
    local commands; commands=()
    _describe -t commands 'cortex config reset commands' commands "$@"
}
(( $+functions[_cortex__config__set_commands] )) ||
_cortex__config__set_commands() {
    local commands; commands=()
    _describe -t commands 'cortex config set commands' commands "$@"
}
(( $+functions[_cortex__config__show_commands] )) ||
_cortex__config__show_commands() {
    local commands; commands=()
    _describe -t commands 'cortex config show commands' commands "$@"
}
(( $+functions[_cortex__debug_commands] )) ||
_cortex__debug_commands() {
    local commands; commands=(
'capsule:Debug context capsule building for a symbol' \
'cache:Show cache statistics' \
'trace:Trace query execution' \
'validate:Validate index integrity' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cortex debug commands' commands "$@"
}
(( $+functions[_cortex__debug__cache_commands] )) ||
_cortex__debug__cache_commands() {
    local commands; commands=()
    _describe -t commands 'cortex debug cache commands' commands "$@"
}
(( $+functions[_cortex__debug__capsule_commands] )) ||
_cortex__debug__capsule_commands() {
    local commands; commands=()
    _describe -t commands 'cortex debug capsule commands' commands "$@"
}
(( $+functions[_cortex__debug__help_commands] )) ||
_cortex__debug__help_commands() {
    local commands; commands=(
'capsule:Debug context capsule building for a symbol' \
'cache:Show cache statistics' \
'trace:Trace query execution' \
'validate:Validate index integrity' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cortex debug help commands' commands "$@"
}
(( $+functions[_cortex__debug__help__cache_commands] )) ||
_cortex__debug__help__cache_commands() {
    local commands; commands=()
    _describe -t commands 'cortex debug help cache commands' commands "$@"
}
(( $+functions[_cortex__debug__help__capsule_commands] )) ||
_cortex__debug__help__capsule_commands() {
    local commands; commands=()
    _describe -t commands 'cortex debug help capsule commands' commands "$@"
}
(( $+functions[_cortex__debug__help__help_commands] )) ||
_cortex__debug__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'cortex debug help help commands' commands "$@"
}
(( $+functions[_cortex__debug__help__trace_commands] )) ||
_cortex__debug__help__trace_commands() {
    local commands; commands=()
    _describe -t commands 'cortex debug help trace commands' commands "$@"
}
(( $+functions[_cortex__debug__help__validate_commands] )) ||
_cortex__debug__help__validate_commands() {
    local commands; commands=()
    _describe -t commands 'cortex debug help validate commands' commands "$@"
}
(( $+functions[_cortex__debug__trace_commands] )) ||
_cortex__debug__trace_commands() {
    local commands; commands=()
    _describe -t commands 'cortex debug trace commands' commands "$@"
}
(( $+functions[_cortex__debug__validate_commands] )) ||
_cortex__debug__validate_commands() {
    local commands; commands=()
    _describe -t commands 'cortex debug validate commands' commands "$@"
}
(( $+functions[_cortex__delete_commands] )) ||
_cortex__delete_commands() {
    local commands; commands=()
    _describe -t commands 'cortex delete commands' commands "$@"
}
(( $+functions[_cortex__doctor_commands] )) ||
_cortex__doctor_commands() {
    local commands; commands=()
    _describe -t commands 'cortex doctor commands' commands "$@"
}
(( $+functions[_cortex__find_commands] )) ||
_cortex__find_commands() {
    local commands; commands=(
'name:' \
'pattern:' \
'type:' \
'content:' \
'decorator:' \
'argument:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cortex find commands' commands "$@"
}
(( $+functions[_cortex__find__argument_commands] )) ||
_cortex__find__argument_commands() {
    local commands; commands=()
    _describe -t commands 'cortex find argument commands' commands "$@"
}
(( $+functions[_cortex__find__content_commands] )) ||
_cortex__find__content_commands() {
    local commands; commands=()
    _describe -t commands 'cortex find content commands' commands "$@"
}
(( $+functions[_cortex__find__decorator_commands] )) ||
_cortex__find__decorator_commands() {
    local commands; commands=()
    _describe -t commands 'cortex find decorator commands' commands "$@"
}
(( $+functions[_cortex__find__help_commands] )) ||
_cortex__find__help_commands() {
    local commands; commands=(
'name:' \
'pattern:' \
'type:' \
'content:' \
'decorator:' \
'argument:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cortex find help commands' commands "$@"
}
(( $+functions[_cortex__find__help__argument_commands] )) ||
_cortex__find__help__argument_commands() {
    local commands; commands=()
    _describe -t commands 'cortex find help argument commands' commands "$@"
}
(( $+functions[_cortex__find__help__content_commands] )) ||
_cortex__find__help__content_commands() {
    local commands; commands=()
    _describe -t commands 'cortex find help content commands' commands "$@"
}
(( $+functions[_cortex__find__help__decorator_commands] )) ||
_cortex__find__help__decorator_commands() {
    local commands; commands=()
    _describe -t commands 'cortex find help decorator commands' commands "$@"
}
(( $+functions[_cortex__find__help__help_commands] )) ||
_cortex__find__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'cortex find help help commands' commands "$@"
}
(( $+functions[_cortex__find__help__name_commands] )) ||
_cortex__find__help__name_commands() {
    local commands; commands=()
    _describe -t commands 'cortex find help name commands' commands "$@"
}
(( $+functions[_cortex__find__help__pattern_commands] )) ||
_cortex__find__help__pattern_commands() {
    local commands; commands=()
    _describe -t commands 'cortex find help pattern commands' commands "$@"
}
(( $+functions[_cortex__find__help__type_commands] )) ||
_cortex__find__help__type_commands() {
    local commands; commands=()
    _describe -t commands 'cortex find help type commands' commands "$@"
}
(( $+functions[_cortex__find__name_commands] )) ||
_cortex__find__name_commands() {
    local commands; commands=()
    _describe -t commands 'cortex find name commands' commands "$@"
}
(( $+functions[_cortex__find__pattern_commands] )) ||
_cortex__find__pattern_commands() {
    local commands; commands=()
    _describe -t commands 'cortex find pattern commands' commands "$@"
}
(( $+functions[_cortex__find__type_commands] )) ||
_cortex__find__type_commands() {
    local commands; commands=()
    _describe -t commands 'cortex find type commands' commands "$@"
}
(( $+functions[_cortex__help_commands] )) ||
_cortex__help_commands() {
    local commands; commands=(
'setup:' \
'doctor:' \
'mcp:' \
'index:' \
'watch:' \
'unwatch:' \
'find:' \
'analyze:' \
'bundle:' \
'config:' \
'clean:' \
'list:' \
'delete:' \
'stats:' \
'query:' \
'jobs:' \
'debug:' \
'completion:Generate shell completion scripts' \
'interactive:Start interactive REPL mode' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cortex help commands' commands "$@"
}
(( $+functions[_cortex__help__analyze_commands] )) ||
_cortex__help__analyze_commands() {
    local commands; commands=(
'callers:' \
'callees:' \
'chain:' \
'hierarchy:' \
'deps:' \
'dead-code:' \
'complexity:' \
'overrides:' \
    )
    _describe -t commands 'cortex help analyze commands' commands "$@"
}
(( $+functions[_cortex__help__analyze__callees_commands] )) ||
_cortex__help__analyze__callees_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help analyze callees commands' commands "$@"
}
(( $+functions[_cortex__help__analyze__callers_commands] )) ||
_cortex__help__analyze__callers_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help analyze callers commands' commands "$@"
}
(( $+functions[_cortex__help__analyze__chain_commands] )) ||
_cortex__help__analyze__chain_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help analyze chain commands' commands "$@"
}
(( $+functions[_cortex__help__analyze__complexity_commands] )) ||
_cortex__help__analyze__complexity_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help analyze complexity commands' commands "$@"
}
(( $+functions[_cortex__help__analyze__dead-code_commands] )) ||
_cortex__help__analyze__dead-code_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help analyze dead-code commands' commands "$@"
}
(( $+functions[_cortex__help__analyze__deps_commands] )) ||
_cortex__help__analyze__deps_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help analyze deps commands' commands "$@"
}
(( $+functions[_cortex__help__analyze__hierarchy_commands] )) ||
_cortex__help__analyze__hierarchy_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help analyze hierarchy commands' commands "$@"
}
(( $+functions[_cortex__help__analyze__overrides_commands] )) ||
_cortex__help__analyze__overrides_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help analyze overrides commands' commands "$@"
}
(( $+functions[_cortex__help__bundle_commands] )) ||
_cortex__help__bundle_commands() {
    local commands; commands=(
'export:' \
'import:' \
    )
    _describe -t commands 'cortex help bundle commands' commands "$@"
}
(( $+functions[_cortex__help__bundle__export_commands] )) ||
_cortex__help__bundle__export_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help bundle export commands' commands "$@"
}
(( $+functions[_cortex__help__bundle__import_commands] )) ||
_cortex__help__bundle__import_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help bundle import commands' commands "$@"
}
(( $+functions[_cortex__help__clean_commands] )) ||
_cortex__help__clean_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help clean commands' commands "$@"
}
(( $+functions[_cortex__help__completion_commands] )) ||
_cortex__help__completion_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help completion commands' commands "$@"
}
(( $+functions[_cortex__help__config_commands] )) ||
_cortex__help__config_commands() {
    local commands; commands=(
'show:' \
'set:' \
'reset:' \
    )
    _describe -t commands 'cortex help config commands' commands "$@"
}
(( $+functions[_cortex__help__config__reset_commands] )) ||
_cortex__help__config__reset_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help config reset commands' commands "$@"
}
(( $+functions[_cortex__help__config__set_commands] )) ||
_cortex__help__config__set_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help config set commands' commands "$@"
}
(( $+functions[_cortex__help__config__show_commands] )) ||
_cortex__help__config__show_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help config show commands' commands "$@"
}
(( $+functions[_cortex__help__debug_commands] )) ||
_cortex__help__debug_commands() {
    local commands; commands=(
'capsule:Debug context capsule building for a symbol' \
'cache:Show cache statistics' \
'trace:Trace query execution' \
'validate:Validate index integrity' \
    )
    _describe -t commands 'cortex help debug commands' commands "$@"
}
(( $+functions[_cortex__help__debug__cache_commands] )) ||
_cortex__help__debug__cache_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help debug cache commands' commands "$@"
}
(( $+functions[_cortex__help__debug__capsule_commands] )) ||
_cortex__help__debug__capsule_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help debug capsule commands' commands "$@"
}
(( $+functions[_cortex__help__debug__trace_commands] )) ||
_cortex__help__debug__trace_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help debug trace commands' commands "$@"
}
(( $+functions[_cortex__help__debug__validate_commands] )) ||
_cortex__help__debug__validate_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help debug validate commands' commands "$@"
}
(( $+functions[_cortex__help__delete_commands] )) ||
_cortex__help__delete_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help delete commands' commands "$@"
}
(( $+functions[_cortex__help__doctor_commands] )) ||
_cortex__help__doctor_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help doctor commands' commands "$@"
}
(( $+functions[_cortex__help__find_commands] )) ||
_cortex__help__find_commands() {
    local commands; commands=(
'name:' \
'pattern:' \
'type:' \
'content:' \
'decorator:' \
'argument:' \
    )
    _describe -t commands 'cortex help find commands' commands "$@"
}
(( $+functions[_cortex__help__find__argument_commands] )) ||
_cortex__help__find__argument_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help find argument commands' commands "$@"
}
(( $+functions[_cortex__help__find__content_commands] )) ||
_cortex__help__find__content_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help find content commands' commands "$@"
}
(( $+functions[_cortex__help__find__decorator_commands] )) ||
_cortex__help__find__decorator_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help find decorator commands' commands "$@"
}
(( $+functions[_cortex__help__find__name_commands] )) ||
_cortex__help__find__name_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help find name commands' commands "$@"
}
(( $+functions[_cortex__help__find__pattern_commands] )) ||
_cortex__help__find__pattern_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help find pattern commands' commands "$@"
}
(( $+functions[_cortex__help__find__type_commands] )) ||
_cortex__help__find__type_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help find type commands' commands "$@"
}
(( $+functions[_cortex__help__help_commands] )) ||
_cortex__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help help commands' commands "$@"
}
(( $+functions[_cortex__help__index_commands] )) ||
_cortex__help__index_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help index commands' commands "$@"
}
(( $+functions[_cortex__help__interactive_commands] )) ||
_cortex__help__interactive_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help interactive commands' commands "$@"
}
(( $+functions[_cortex__help__jobs_commands] )) ||
_cortex__help__jobs_commands() {
    local commands; commands=(
'list:' \
'status:' \
    )
    _describe -t commands 'cortex help jobs commands' commands "$@"
}
(( $+functions[_cortex__help__jobs__list_commands] )) ||
_cortex__help__jobs__list_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help jobs list commands' commands "$@"
}
(( $+functions[_cortex__help__jobs__status_commands] )) ||
_cortex__help__jobs__status_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help jobs status commands' commands "$@"
}
(( $+functions[_cortex__help__list_commands] )) ||
_cortex__help__list_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help list commands' commands "$@"
}
(( $+functions[_cortex__help__mcp_commands] )) ||
_cortex__help__mcp_commands() {
    local commands; commands=(
'start:' \
'tools:' \
    )
    _describe -t commands 'cortex help mcp commands' commands "$@"
}
(( $+functions[_cortex__help__mcp__start_commands] )) ||
_cortex__help__mcp__start_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help mcp start commands' commands "$@"
}
(( $+functions[_cortex__help__mcp__tools_commands] )) ||
_cortex__help__mcp__tools_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help mcp tools commands' commands "$@"
}
(( $+functions[_cortex__help__query_commands] )) ||
_cortex__help__query_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help query commands' commands "$@"
}
(( $+functions[_cortex__help__setup_commands] )) ||
_cortex__help__setup_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help setup commands' commands "$@"
}
(( $+functions[_cortex__help__stats_commands] )) ||
_cortex__help__stats_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help stats commands' commands "$@"
}
(( $+functions[_cortex__help__unwatch_commands] )) ||
_cortex__help__unwatch_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help unwatch commands' commands "$@"
}
(( $+functions[_cortex__help__watch_commands] )) ||
_cortex__help__watch_commands() {
    local commands; commands=()
    _describe -t commands 'cortex help watch commands' commands "$@"
}
(( $+functions[_cortex__index_commands] )) ||
_cortex__index_commands() {
    local commands; commands=()
    _describe -t commands 'cortex index commands' commands "$@"
}
(( $+functions[_cortex__interactive_commands] )) ||
_cortex__interactive_commands() {
    local commands; commands=()
    _describe -t commands 'cortex interactive commands' commands "$@"
}
(( $+functions[_cortex__jobs_commands] )) ||
_cortex__jobs_commands() {
    local commands; commands=(
'list:' \
'status:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cortex jobs commands' commands "$@"
}
(( $+functions[_cortex__jobs__help_commands] )) ||
_cortex__jobs__help_commands() {
    local commands; commands=(
'list:' \
'status:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cortex jobs help commands' commands "$@"
}
(( $+functions[_cortex__jobs__help__help_commands] )) ||
_cortex__jobs__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'cortex jobs help help commands' commands "$@"
}
(( $+functions[_cortex__jobs__help__list_commands] )) ||
_cortex__jobs__help__list_commands() {
    local commands; commands=()
    _describe -t commands 'cortex jobs help list commands' commands "$@"
}
(( $+functions[_cortex__jobs__help__status_commands] )) ||
_cortex__jobs__help__status_commands() {
    local commands; commands=()
    _describe -t commands 'cortex jobs help status commands' commands "$@"
}
(( $+functions[_cortex__jobs__list_commands] )) ||
_cortex__jobs__list_commands() {
    local commands; commands=()
    _describe -t commands 'cortex jobs list commands' commands "$@"
}
(( $+functions[_cortex__jobs__status_commands] )) ||
_cortex__jobs__status_commands() {
    local commands; commands=()
    _describe -t commands 'cortex jobs status commands' commands "$@"
}
(( $+functions[_cortex__list_commands] )) ||
_cortex__list_commands() {
    local commands; commands=()
    _describe -t commands 'cortex list commands' commands "$@"
}
(( $+functions[_cortex__mcp_commands] )) ||
_cortex__mcp_commands() {
    local commands; commands=(
'start:' \
'tools:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cortex mcp commands' commands "$@"
}
(( $+functions[_cortex__mcp__help_commands] )) ||
_cortex__mcp__help_commands() {
    local commands; commands=(
'start:' \
'tools:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'cortex mcp help commands' commands "$@"
}
(( $+functions[_cortex__mcp__help__help_commands] )) ||
_cortex__mcp__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'cortex mcp help help commands' commands "$@"
}
(( $+functions[_cortex__mcp__help__start_commands] )) ||
_cortex__mcp__help__start_commands() {
    local commands; commands=()
    _describe -t commands 'cortex mcp help start commands' commands "$@"
}
(( $+functions[_cortex__mcp__help__tools_commands] )) ||
_cortex__mcp__help__tools_commands() {
    local commands; commands=()
    _describe -t commands 'cortex mcp help tools commands' commands "$@"
}
(( $+functions[_cortex__mcp__start_commands] )) ||
_cortex__mcp__start_commands() {
    local commands; commands=()
    _describe -t commands 'cortex mcp start commands' commands "$@"
}
(( $+functions[_cortex__mcp__tools_commands] )) ||
_cortex__mcp__tools_commands() {
    local commands; commands=()
    _describe -t commands 'cortex mcp tools commands' commands "$@"
}
(( $+functions[_cortex__query_commands] )) ||
_cortex__query_commands() {
    local commands; commands=()
    _describe -t commands 'cortex query commands' commands "$@"
}
(( $+functions[_cortex__setup_commands] )) ||
_cortex__setup_commands() {
    local commands; commands=()
    _describe -t commands 'cortex setup commands' commands "$@"
}
(( $+functions[_cortex__stats_commands] )) ||
_cortex__stats_commands() {
    local commands; commands=()
    _describe -t commands 'cortex stats commands' commands "$@"
}
(( $+functions[_cortex__unwatch_commands] )) ||
_cortex__unwatch_commands() {
    local commands; commands=()
    _describe -t commands 'cortex unwatch commands' commands "$@"
}
(( $+functions[_cortex__watch_commands] )) ||
_cortex__watch_commands() {
    local commands; commands=()
    _describe -t commands 'cortex watch commands' commands "$@"
}

if [ "$funcstack[1]" = "_cortex" ]; then
    _cortex "$@"
else
    compdef _cortex cortex
fi
