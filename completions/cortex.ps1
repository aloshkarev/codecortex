
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
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
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
            [CompletionResult]::new('capsule', 'capsule', [CompletionResultType]::ParameterValue, 'Get context capsule for a symbol')
            [CompletionResult]::new('impact', 'impact', [CompletionResultType]::ParameterValue, 'Get impact graph for a symbol')
            [CompletionResult]::new('refactor', 'refactor', [CompletionResultType]::ParameterValue, 'Analyze refactoring suggestions for a symbol')
            [CompletionResult]::new('patterns', 'patterns', [CompletionResultType]::ParameterValue, 'Find design patterns in codebase')
            [CompletionResult]::new('test', 'test', [CompletionResultType]::ParameterValue, 'Find tests for a symbol')
            [CompletionResult]::new('diagnose', 'diagnose', [CompletionResultType]::ParameterValue, 'Run diagnostic checks')
            [CompletionResult]::new('memory', 'memory', [CompletionResultType]::ParameterValue, 'Memory operations')
            [CompletionResult]::new('project', 'project', [CompletionResultType]::ParameterValue, 'Project management operations')
            [CompletionResult]::new('skeleton', 'skeleton', [CompletionResultType]::ParameterValue, 'Get skeleton (compressed view) of a file')
            [CompletionResult]::new('signature', 'signature', [CompletionResultType]::ParameterValue, 'Get signature of a symbol')
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'Semantic code search using vector embeddings')
            [CompletionResult]::new('vector-index', 'vector-index', [CompletionResultType]::ParameterValue, 'Index code for vector search')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;setup' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;doctor' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;mcp' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('start', 'start', [CompletionResultType]::ParameterValue, 'start')
            [CompletionResult]::new('tools', 'tools', [CompletionResultType]::ParameterValue, 'tools')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;mcp;start' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;mcp;tools' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
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
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('--force', '--force', [CompletionResultType]::ParameterName, 'force')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;watch' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;unwatch' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;find' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
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
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;find;pattern' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;find;type' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;find;content' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;find;decorator' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;find;argument' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
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
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
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
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;analyze;callees' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;analyze;chain' {
            [CompletionResult]::new('--depth', '--depth', [CompletionResultType]::ParameterName, 'depth')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;analyze;hierarchy' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;analyze;deps' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;analyze;dead-code' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;analyze;complexity' {
            [CompletionResult]::new('--top', '--top', [CompletionResultType]::ParameterName, 'top')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;analyze;overrides' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
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
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'export')
            [CompletionResult]::new('import', 'import', [CompletionResultType]::ParameterValue, 'import')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;bundle;export' {
            [CompletionResult]::new('--repo', '--repo', [CompletionResultType]::ParameterName, 'repo')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;bundle;import' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
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
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'show')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'set')
            [CompletionResult]::new('reset', 'reset', [CompletionResultType]::ParameterValue, 'reset')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;config;show' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;config;set' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;config;reset' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
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
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;list' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;delete' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;stats' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;query' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;jobs' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'list')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'status')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;jobs;list' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;jobs;status' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
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
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('capsule', 'capsule', [CompletionResultType]::ParameterValue, 'Debug context capsule building for a symbol')
            [CompletionResult]::new('cache', 'cache', [CompletionResultType]::ParameterValue, 'Show cache statistics')
            [CompletionResult]::new('trace', 'trace', [CompletionResultType]::ParameterValue, 'Trace query execution')
            [CompletionResult]::new('validate', 'validate', [CompletionResultType]::ParameterValue, 'Validate index integrity')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;debug;capsule' {
            [CompletionResult]::new('--max-items', '--max-items', [CompletionResultType]::ParameterName, 'Maximum items in capsule')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('--explain', '--explain', [CompletionResultType]::ParameterName, 'Explain the capsule building process')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;debug;cache' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('--clear', '--clear', [CompletionResultType]::ParameterName, 'Clear the cache')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;debug;trace' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose output')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;debug;validate' {
            [CompletionResult]::new('--repo', '--repo', [CompletionResultType]::ParameterName, 'Repository path to validate')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('--fix', '--fix', [CompletionResultType]::ParameterName, 'Fix issues automatically')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
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
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;interactive' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;capsule' {
            [CompletionResult]::new('--max-items', '--max-items', [CompletionResultType]::ParameterName, 'Maximum items in capsule')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;impact' {
            [CompletionResult]::new('--depth', '--depth', [CompletionResultType]::ParameterName, 'Maximum depth to traverse')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;refactor' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;patterns' {
            [CompletionResult]::new('--pattern-type', '--pattern-type', [CompletionResultType]::ParameterName, 'Filter by pattern type (singleton, factory, observer, etc.)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;test' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;diagnose' {
            [CompletionResult]::new('--component', '--component', [CompletionResultType]::ParameterName, 'Check specific component')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;memory' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('save', 'save', [CompletionResultType]::ParameterValue, 'Save an observation')
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'Search observations')
            [CompletionResult]::new('context', 'context', [CompletionResultType]::ParameterValue, 'Get session context')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all observations')
            [CompletionResult]::new('clear', 'clear', [CompletionResultType]::ParameterValue, 'Clear all observations')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;memory;save' {
            [CompletionResult]::new('--classification', '--classification', [CompletionResultType]::ParameterName, 'Classification (architecture, decision, pattern, issue, note)')
            [CompletionResult]::new('--severity', '--severity', [CompletionResultType]::ParameterName, 'Severity (low, medium, high, critical)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;memory;search' {
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'Maximum results')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;memory;context' {
            [CompletionResult]::new('--session', '--session', [CompletionResultType]::ParameterName, 'Session ID (optional)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;memory;list' {
            [CompletionResult]::new('--classification', '--classification', [CompletionResultType]::ParameterName, 'Filter by classification')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;memory;clear' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;memory;help' {
            [CompletionResult]::new('save', 'save', [CompletionResultType]::ParameterValue, 'Save an observation')
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'Search observations')
            [CompletionResult]::new('context', 'context', [CompletionResultType]::ParameterValue, 'Get session context')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all observations')
            [CompletionResult]::new('clear', 'clear', [CompletionResultType]::ParameterValue, 'Clear all observations')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;memory;help;save' {
            break
        }
        'cortex;memory;help;search' {
            break
        }
        'cortex;memory;help;context' {
            break
        }
        'cortex;memory;help;list' {
            break
        }
        'cortex;memory;help;clear' {
            break
        }
        'cortex;memory;help;help' {
            break
        }
        'cortex;project' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all registered projects')
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add a project to the registry')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove a project from the registry')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set the current active project')
            [CompletionResult]::new('current', 'current', [CompletionResultType]::ParameterValue, 'Get the current active project')
            [CompletionResult]::new('branches', 'branches', [CompletionResultType]::ParameterValue, 'List branches for a project')
            [CompletionResult]::new('refresh', 'refresh', [CompletionResultType]::ParameterValue, 'Refresh Git info for a project')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;project;list' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;project;add' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('--track-branch', '--track-branch', [CompletionResultType]::ParameterName, 'Whether to track branch changes')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;project;remove' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;project;set' {
            [CompletionResult]::new('--branch', '--branch', [CompletionResultType]::ParameterName, 'Branch to use (optional, defaults to current)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;project;current' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;project;branches' {
            [CompletionResult]::new('--path', '--path', [CompletionResultType]::ParameterName, 'Path to the project (optional, uses current)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;project;refresh' {
            [CompletionResult]::new('--path', '--path', [CompletionResultType]::ParameterName, 'Path to the project (optional, uses current)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;project;help' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all registered projects')
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add a project to the registry')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove a project from the registry')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set the current active project')
            [CompletionResult]::new('current', 'current', [CompletionResultType]::ParameterValue, 'Get the current active project')
            [CompletionResult]::new('branches', 'branches', [CompletionResultType]::ParameterValue, 'List branches for a project')
            [CompletionResult]::new('refresh', 'refresh', [CompletionResultType]::ParameterValue, 'Refresh Git info for a project')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'cortex;project;help;list' {
            break
        }
        'cortex;project;help;add' {
            break
        }
        'cortex;project;help;remove' {
            break
        }
        'cortex;project;help;set' {
            break
        }
        'cortex;project;help;current' {
            break
        }
        'cortex;project;help;branches' {
            break
        }
        'cortex;project;help;refresh' {
            break
        }
        'cortex;project;help;help' {
            break
        }
        'cortex;skeleton' {
            [CompletionResult]::new('--mode', '--mode', [CompletionResultType]::ParameterName, 'Skeleton mode (minimal, standard, full)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;signature' {
            [CompletionResult]::new('--repo', '--repo', [CompletionResultType]::ParameterName, 'Repository path filter')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('--include-related', '--include-related', [CompletionResultType]::ParameterName, 'Include related symbols')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;search' {
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'Number of results to return')
            [CompletionResult]::new('--search-type', '--search-type', [CompletionResultType]::ParameterName, 'Search type (semantic, structural, hybrid)')
            [CompletionResult]::new('--repo', '--repo', [CompletionResultType]::ParameterName, 'Filter by repository path')
            [CompletionResult]::new('--path', '--path', [CompletionResultType]::ParameterName, 'Filter by file path pattern')
            [CompletionResult]::new('--kind', '--kind', [CompletionResultType]::ParameterName, 'Filter by symbol kind (function, class, method, etc.)')
            [CompletionResult]::new('--language', '--language', [CompletionResultType]::ParameterName, 'Filter by language')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'cortex;vector-index' {
            [CompletionResult]::new('--repo', '--repo', [CompletionResultType]::ParameterName, 'Repository path for metadata')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (format, json-pretty, yaml, table)')
            [CompletionResult]::new('--force', '--force', [CompletionResultType]::ParameterName, 'Force reindex')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
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
            [CompletionResult]::new('capsule', 'capsule', [CompletionResultType]::ParameterValue, 'Get context capsule for a symbol')
            [CompletionResult]::new('impact', 'impact', [CompletionResultType]::ParameterValue, 'Get impact graph for a symbol')
            [CompletionResult]::new('refactor', 'refactor', [CompletionResultType]::ParameterValue, 'Analyze refactoring suggestions for a symbol')
            [CompletionResult]::new('patterns', 'patterns', [CompletionResultType]::ParameterValue, 'Find design patterns in codebase')
            [CompletionResult]::new('test', 'test', [CompletionResultType]::ParameterValue, 'Find tests for a symbol')
            [CompletionResult]::new('diagnose', 'diagnose', [CompletionResultType]::ParameterValue, 'Run diagnostic checks')
            [CompletionResult]::new('memory', 'memory', [CompletionResultType]::ParameterValue, 'Memory operations')
            [CompletionResult]::new('project', 'project', [CompletionResultType]::ParameterValue, 'Project management operations')
            [CompletionResult]::new('skeleton', 'skeleton', [CompletionResultType]::ParameterValue, 'Get skeleton (compressed view) of a file')
            [CompletionResult]::new('signature', 'signature', [CompletionResultType]::ParameterValue, 'Get signature of a symbol')
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'Semantic code search using vector embeddings')
            [CompletionResult]::new('vector-index', 'vector-index', [CompletionResultType]::ParameterValue, 'Index code for vector search')
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
        'cortex;help;capsule' {
            break
        }
        'cortex;help;impact' {
            break
        }
        'cortex;help;refactor' {
            break
        }
        'cortex;help;patterns' {
            break
        }
        'cortex;help;test' {
            break
        }
        'cortex;help;diagnose' {
            break
        }
        'cortex;help;memory' {
            [CompletionResult]::new('save', 'save', [CompletionResultType]::ParameterValue, 'Save an observation')
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'Search observations')
            [CompletionResult]::new('context', 'context', [CompletionResultType]::ParameterValue, 'Get session context')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all observations')
            [CompletionResult]::new('clear', 'clear', [CompletionResultType]::ParameterValue, 'Clear all observations')
            break
        }
        'cortex;help;memory;save' {
            break
        }
        'cortex;help;memory;search' {
            break
        }
        'cortex;help;memory;context' {
            break
        }
        'cortex;help;memory;list' {
            break
        }
        'cortex;help;memory;clear' {
            break
        }
        'cortex;help;project' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all registered projects')
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add a project to the registry')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove a project from the registry')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set the current active project')
            [CompletionResult]::new('current', 'current', [CompletionResultType]::ParameterValue, 'Get the current active project')
            [CompletionResult]::new('branches', 'branches', [CompletionResultType]::ParameterValue, 'List branches for a project')
            [CompletionResult]::new('refresh', 'refresh', [CompletionResultType]::ParameterValue, 'Refresh Git info for a project')
            break
        }
        'cortex;help;project;list' {
            break
        }
        'cortex;help;project;add' {
            break
        }
        'cortex;help;project;remove' {
            break
        }
        'cortex;help;project;set' {
            break
        }
        'cortex;help;project;current' {
            break
        }
        'cortex;help;project;branches' {
            break
        }
        'cortex;help;project;refresh' {
            break
        }
        'cortex;help;skeleton' {
            break
        }
        'cortex;help;signature' {
            break
        }
        'cortex;help;search' {
            break
        }
        'cortex;help;vector-index' {
            break
        }
        'cortex;help;help' {
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
