
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'cortex' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'cortex'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'cortex' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('setup', 'setup', [CompletionResultType]::ParameterValue, 'setup')
            [CompletionResult]::new('doctor', 'doctor', [CompletionResultType]::ParameterValue, 'doctor')
            [CompletionResult]::new('mcp', 'mcp', [CompletionResultType]::ParameterValue, 'mcp')
            [CompletionResult]::new('index', 'index', [CompletionResultType]::ParameterValue, 'index')
            [CompletionResult]::new('watch', 'watch', [CompletionResultType]::ParameterValue, 'watch')
            [CompletionResult]::new('unwatch', 'unwatch', [CompletionResultType]::ParameterValue, 'unwatch')
            [CompletionResult]::new('find', 'find', [CompletionResultType]::ParameterValue, 'find')
            [CompletionResult]::new('analyze', 'analyze', [CompletionResultType]::ParameterValue, 'analyze')
            [CompletionResult]::new('bundle', 'bundle', [CompletionResultType]::ParameterValue, 'bundle')
            [CompletionResult]::new('config', 'config', [CompletionResultType]::ParameterValue, 'config')
            [CompletionResult]::new('clean', 'clean', [CompletionResultType]::ParameterValue, 'clean')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'list')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'delete')
            [CompletionResult]::new('stats', 'stats', [CompletionResultType]::ParameterValue, 'stats')
            [CompletionResult]::new('query', 'query', [CompletionResultType]::ParameterValue, 'query')
            [CompletionResult]::new('jobs', 'jobs', [CompletionResultType]::ParameterValue, 'jobs')
            [CompletionResult]::new('debug', 'debug', [CompletionResultType]::ParameterValue, 'debug')
            [CompletionResult]::new('completion', 'completion', [CompletionResultType]::ParameterValue, 'Generate shell completion scripts')
            [CompletionResult]::new('interactive', 'interactive', [CompletionResultType]::ParameterValue, 'Start interactive REPL mode')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;setup' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;doctor' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;mcp' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('start', 'start', [CompletionResultType]::ParameterValue, 'start')
            [CompletionResult]::new('tools', 'tools', [CompletionResultType]::ParameterValue, 'tools')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;mcp;start' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;mcp;tools' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;mcp;help' {
            [CompletionResult]::new('start', 'start', [CompletionResultType]::ParameterValue, 'start')
            [CompletionResult]::new('tools', 'tools', [CompletionResultType]::ParameterValue, 'tools')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;mcp;help;start' {
            break
        }
        'cortex;mcp;help;tools' {
            break
        }
        'cortex;mcp;help;help' {
            break
        }
        'cortex;index' {
            [CompletionResult]::new('--force', '--force', [CompletionResultType]::ParameterName, 'force')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;watch' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;unwatch' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;find' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('name', 'name', [CompletionResultType]::ParameterValue, 'name')
            [CompletionResult]::new('pattern', 'pattern', [CompletionResultType]::ParameterValue, 'pattern')
            [CompletionResult]::new('type', 'type', [CompletionResultType]::ParameterValue, 'type')
            [CompletionResult]::new('content', 'content', [CompletionResultType]::ParameterValue, 'content')
            [CompletionResult]::new('decorator', 'decorator', [CompletionResultType]::ParameterValue, 'decorator')
            [CompletionResult]::new('argument', 'argument', [CompletionResultType]::ParameterValue, 'argument')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;find;name' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;find;pattern' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;find;type' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;find;content' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;find;decorator' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;find;argument' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;find;help' {
            [CompletionResult]::new('name', 'name', [CompletionResultType]::ParameterValue, 'name')
            [CompletionResult]::new('pattern', 'pattern', [CompletionResultType]::ParameterValue, 'pattern')
            [CompletionResult]::new('type', 'type', [CompletionResultType]::ParameterValue, 'type')
            [CompletionResult]::new('content', 'content', [CompletionResultType]::ParameterValue, 'content')
            [CompletionResult]::new('decorator', 'decorator', [CompletionResultType]::ParameterValue, 'decorator')
            [CompletionResult]::new('argument', 'argument', [CompletionResultType]::ParameterValue, 'argument')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;find;help;name' {
            break
        }
        'cortex;find;help;pattern' {
            break
        }
        'cortex;find;help;type' {
            break
        }
        'cortex;find;help;content' {
            break
        }
        'cortex;find;help;decorator' {
            break
        }
        'cortex;find;help;argument' {
            break
        }
        'cortex;find;help;help' {
            break
        }
        'cortex;analyze' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('callers', 'callers', [CompletionResultType]::ParameterValue, 'callers')
            [CompletionResult]::new('callees', 'callees', [CompletionResultType]::ParameterValue, 'callees')
            [CompletionResult]::new('chain', 'chain', [CompletionResultType]::ParameterValue, 'chain')
            [CompletionResult]::new('hierarchy', 'hierarchy', [CompletionResultType]::ParameterValue, 'hierarchy')
            [CompletionResult]::new('deps', 'deps', [CompletionResultType]::ParameterValue, 'deps')
            [CompletionResult]::new('dead-code', 'dead-code', [CompletionResultType]::ParameterValue, 'dead-code')
            [CompletionResult]::new('complexity', 'complexity', [CompletionResultType]::ParameterValue, 'complexity')
            [CompletionResult]::new('overrides', 'overrides', [CompletionResultType]::ParameterValue, 'overrides')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;analyze;callers' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;analyze;callees' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;analyze;chain' {
            [CompletionResult]::new('--depth', '--depth', [CompletionResultType]::ParameterName, 'depth')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;analyze;hierarchy' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;analyze;deps' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;analyze;dead-code' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;analyze;complexity' {
            [CompletionResult]::new('--top', '--top', [CompletionResultType]::ParameterName, 'top')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;analyze;overrides' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;analyze;help' {
            [CompletionResult]::new('callers', 'callers', [CompletionResultType]::ParameterValue, 'callers')
            [CompletionResult]::new('callees', 'callees', [CompletionResultType]::ParameterValue, 'callees')
            [CompletionResult]::new('chain', 'chain', [CompletionResultType]::ParameterValue, 'chain')
            [CompletionResult]::new('hierarchy', 'hierarchy', [CompletionResultType]::ParameterValue, 'hierarchy')
            [CompletionResult]::new('deps', 'deps', [CompletionResultType]::ParameterValue, 'deps')
            [CompletionResult]::new('dead-code', 'dead-code', [CompletionResultType]::ParameterValue, 'dead-code')
            [CompletionResult]::new('complexity', 'complexity', [CompletionResultType]::ParameterValue, 'complexity')
            [CompletionResult]::new('overrides', 'overrides', [CompletionResultType]::ParameterValue, 'overrides')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;analyze;help;callers' {
            break
        }
        'cortex;analyze;help;callees' {
            break
        }
        'cortex;analyze;help;chain' {
            break
        }
        'cortex;analyze;help;hierarchy' {
            break
        }
        'cortex;analyze;help;deps' {
            break
        }
        'cortex;analyze;help;dead-code' {
            break
        }
        'cortex;analyze;help;complexity' {
            break
        }
        'cortex;analyze;help;overrides' {
            break
        }
        'cortex;analyze;help;help' {
            break
        }
        'cortex;bundle' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'export')
            [CompletionResult]::new('import', 'import', [CompletionResultType]::ParameterValue, 'import')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;bundle;export' {
            [CompletionResult]::new('--repo', '--repo', [CompletionResultType]::ParameterName, 'repo')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;bundle;import' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;bundle;help' {
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'export')
            [CompletionResult]::new('import', 'import', [CompletionResultType]::ParameterValue, 'import')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;bundle;help;export' {
            break
        }
        'cortex;bundle;help;import' {
            break
        }
        'cortex;bundle;help;help' {
            break
        }
        'cortex;config' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'show')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'set')
            [CompletionResult]::new('reset', 'reset', [CompletionResultType]::ParameterValue, 'reset')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;config;show' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;config;set' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;config;reset' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;config;help' {
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'show')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'set')
            [CompletionResult]::new('reset', 'reset', [CompletionResultType]::ParameterValue, 'reset')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;config;help;show' {
            break
        }
        'cortex;config;help;set' {
            break
        }
        'cortex;config;help;reset' {
            break
        }
        'cortex;config;help;help' {
            break
        }
        'cortex;clean' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;list' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;delete' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;stats' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;query' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;jobs' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'list')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'status')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;jobs;list' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;jobs;status' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;jobs;help' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'list')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'status')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;jobs;help;list' {
            break
        }
        'cortex;jobs;help;status' {
            break
        }
        'cortex;jobs;help;help' {
            break
        }
        'cortex;debug' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('capsule', 'capsule', [CompletionResultType]::ParameterValue, 'Debug context capsule building for a symbol')
            [CompletionResult]::new('cache', 'cache', [CompletionResultType]::ParameterValue, 'Show cache statistics')
            [CompletionResult]::new('trace', 'trace', [CompletionResultType]::ParameterValue, 'Trace query execution')
            [CompletionResult]::new('validate', 'validate', [CompletionResultType]::ParameterValue, 'Validate index integrity')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;debug;capsule' {
            [CompletionResult]::new('--max-items', '--max-items', [CompletionResultType]::ParameterName, 'Maximum items in capsule')
            [CompletionResult]::new('--explain', '--explain', [CompletionResultType]::ParameterName, 'Explain the capsule building process')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;debug;cache' {
            [CompletionResult]::new('--clear', '--clear', [CompletionResultType]::ParameterName, 'Clear the cache')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;debug;trace' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;debug;validate' {
            [CompletionResult]::new('--repo', '--repo', [CompletionResultType]::ParameterName, 'Repository path to validate')
            [CompletionResult]::new('--fix', '--fix', [CompletionResultType]::ParameterName, 'Fix issues automatically')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;debug;help' {
            [CompletionResult]::new('capsule', 'capsule', [CompletionResultType]::ParameterValue, 'Debug context capsule building for a symbol')
            [CompletionResult]::new('cache', 'cache', [CompletionResultType]::ParameterValue, 'Show cache statistics')
            [CompletionResult]::new('trace', 'trace', [CompletionResultType]::ParameterValue, 'Trace query execution')
            [CompletionResult]::new('validate', 'validate', [CompletionResultType]::ParameterValue, 'Validate index integrity')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;debug;help;capsule' {
            break
        }
        'cortex;debug;help;cache' {
            break
        }
        'cortex;debug;help;trace' {
            break
        }
        'cortex;debug;help;validate' {
            break
        }
        'cortex;debug;help;help' {
            break
        }
        'cortex;completion' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;interactive' {
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'cortex;help' {
            [CompletionResult]::new('setup', 'setup', [CompletionResultType]::ParameterValue, 'setup')
            [CompletionResult]::new('doctor', 'doctor', [CompletionResultType]::ParameterValue, 'doctor')
            [CompletionResult]::new('mcp', 'mcp', [CompletionResultType]::ParameterValue, 'mcp')
            [CompletionResult]::new('index', 'index', [CompletionResultType]::ParameterValue, 'index')
            [CompletionResult]::new('watch', 'watch', [CompletionResultType]::ParameterValue, 'watch')
            [CompletionResult]::new('unwatch', 'unwatch', [CompletionResultType]::ParameterValue, 'unwatch')
            [CompletionResult]::new('find', 'find', [CompletionResultType]::ParameterValue, 'find')
            [CompletionResult]::new('analyze', 'analyze', [CompletionResultType]::ParameterValue, 'analyze')
            [CompletionResult]::new('bundle', 'bundle', [CompletionResultType]::ParameterValue, 'bundle')
            [CompletionResult]::new('config', 'config', [CompletionResultType]::ParameterValue, 'config')
            [CompletionResult]::new('clean', 'clean', [CompletionResultType]::ParameterValue, 'clean')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'list')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'delete')
            [CompletionResult]::new('stats', 'stats', [CompletionResultType]::ParameterValue, 'stats')
            [CompletionResult]::new('query', 'query', [CompletionResultType]::ParameterValue, 'query')
            [CompletionResult]::new('jobs', 'jobs', [CompletionResultType]::ParameterValue, 'jobs')
            [CompletionResult]::new('debug', 'debug', [CompletionResultType]::ParameterValue, 'debug')
            [CompletionResult]::new('completion', 'completion', [CompletionResultType]::ParameterValue, 'Generate shell completion scripts')
            [CompletionResult]::new('interactive', 'interactive', [CompletionResultType]::ParameterValue, 'Start interactive REPL mode')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;help;setup' {
            break
        }
        'cortex;help;doctor' {
            break
        }
        'cortex;help;mcp' {
            [CompletionResult]::new('start', 'start', [CompletionResultType]::ParameterValue, 'start')
            [CompletionResult]::new('tools', 'tools', [CompletionResultType]::ParameterValue, 'tools')
            break
        }
        'cortex;help;mcp;start' {
            break
        }
        'cortex;help;mcp;tools' {
            break
        }
        'cortex;help;index' {
            break
        }
        'cortex;help;watch' {
            break
        }
        'cortex;help;unwatch' {
            break
        }
        'cortex;help;find' {
            [CompletionResult]::new('name', 'name', [CompletionResultType]::ParameterValue, 'name')
            [CompletionResult]::new('pattern', 'pattern', [CompletionResultType]::ParameterValue, 'pattern')
            [CompletionResult]::new('type', 'type', [CompletionResultType]::ParameterValue, 'type')
            [CompletionResult]::new('content', 'content', [CompletionResultType]::ParameterValue, 'content')
            [CompletionResult]::new('decorator', 'decorator', [CompletionResultType]::ParameterValue, 'decorator')
            [CompletionResult]::new('argument', 'argument', [CompletionResultType]::ParameterValue, 'argument')
            break
        }
        'cortex;help;find;name' {
            break
        }
        'cortex;help;find;pattern' {
            break
        }
        'cortex;help;find;type' {
            break
        }
        'cortex;help;find;content' {
            break
        }
        'cortex;help;find;decorator' {
            break
        }
        'cortex;help;find;argument' {
            break
        }
        'cortex;help;analyze' {
            [CompletionResult]::new('callers', 'callers', [CompletionResultType]::ParameterValue, 'callers')
            [CompletionResult]::new('callees', 'callees', [CompletionResultType]::ParameterValue, 'callees')
            [CompletionResult]::new('chain', 'chain', [CompletionResultType]::ParameterValue, 'chain')
            [CompletionResult]::new('hierarchy', 'hierarchy', [CompletionResultType]::ParameterValue, 'hierarchy')
            [CompletionResult]::new('deps', 'deps', [CompletionResultType]::ParameterValue, 'deps')
            [CompletionResult]::new('dead-code', 'dead-code', [CompletionResultType]::ParameterValue, 'dead-code')
            [CompletionResult]::new('complexity', 'complexity', [CompletionResultType]::ParameterValue, 'complexity')
            [CompletionResult]::new('overrides', 'overrides', [CompletionResultType]::ParameterValue, 'overrides')
            break
        }
        'cortex;help;analyze;callers' {
            break
        }
        'cortex;help;analyze;callees' {
            break
        }
        'cortex;help;analyze;chain' {
            break
        }
        'cortex;help;analyze;hierarchy' {
            break
        }
        'cortex;help;analyze;deps' {
            break
        }
        'cortex;help;analyze;dead-code' {
            break
        }
        'cortex;help;analyze;complexity' {
            break
        }
        'cortex;help;analyze;overrides' {
            break
        }
        'cortex;help;bundle' {
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'export')
            [CompletionResult]::new('import', 'import', [CompletionResultType]::ParameterValue, 'import')
            break
        }
        'cortex;help;bundle;export' {
            break
        }
        'cortex;help;bundle;import' {
            break
        }
        'cortex;help;config' {
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'show')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'set')
            [CompletionResult]::new('reset', 'reset', [CompletionResultType]::ParameterValue, 'reset')
            break
        }
        'cortex;help;config;show' {
            break
        }
        'cortex;help;config;set' {
            break
        }
        'cortex;help;config;reset' {
            break
        }
        'cortex;help;clean' {
            break
        }
        'cortex;help;list' {
            break
        }
        'cortex;help;delete' {
            break
        }
        'cortex;help;stats' {
            break
        }
        'cortex;help;query' {
            break
        }
        'cortex;help;jobs' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'list')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'status')
            break
        }
        'cortex;help;jobs;list' {
            break
        }
        'cortex;help;jobs;status' {
            break
        }
        'cortex;help;debug' {
            [CompletionResult]::new('capsule', 'capsule', [CompletionResultType]::ParameterValue, 'Debug context capsule building for a symbol')
            [CompletionResult]::new('cache', 'cache', [CompletionResultType]::ParameterValue, 'Show cache statistics')
            [CompletionResult]::new('trace', 'trace', [CompletionResultType]::ParameterValue, 'Trace query execution')
            [CompletionResult]::new('validate', 'validate', [CompletionResultType]::ParameterValue, 'Validate index integrity')
            break
        }
        'cortex;help;debug;capsule' {
            break
        }
        'cortex;help;debug;cache' {
            break
        }
        'cortex;help;debug;trace' {
            break
        }
        'cortex;help;debug;validate' {
            break
        }
        'cortex;help;completion' {
            break
        }
        'cortex;help;interactive' {
            break
        }
        'cortex;help;help' {
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
