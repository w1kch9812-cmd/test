    $frontendE2eRunnerTaskEntry = if ($MissingRegisteredRunnerTask) {
        ""
    } else {
        @'
    {
      "id": "frontend-e2e",
      "owner": "build-platform",
      "reason": "fixture",
      "required_commands": ["pnpm"],
      "required_services": []
    },
'@
    }
    $duplicateRunnerTaskEntry = if ($DuplicateRunnerTaskRegistry) {
        @'
    {
      "id": "node-audit",
      "owner": "build-platform",
      "reason": "fixture duplicate",
      "required_commands": ["pnpm"],
      "required_services": []
    },
'@
    } else {
        ""
    }
    $pnpmRequiredCommandEntry = if ($MissingRegisteredRequiredCommand) {
        ""
    } else {
        @'
    {
      "id": "pnpm",
      "owner": "build-platform",
      "reason": "fixture"
    },
'@
    }
    $duplicateRequiredCommandEntry = if ($DuplicateRequiredCommandRegistry) {
        @'
    {
      "id": "cargo",
      "owner": "build-platform",
      "reason": "fixture duplicate"
    },
'@
    } else {
        ""
    }
    $registeredRequiredCommands = if ($MissingRequiredCommandRegistry) {
        ""
    } else {
        @"
  "required_command_registry": [
    {
      "id": "cargo",
      "owner": "build-platform",
      "reason": "fixture"
    },
$duplicateRequiredCommandEntry    {
      "id": "pg_isready",
      "owner": "build-platform",
      "reason": "fixture"
    },
$pnpmRequiredCommandEntry    {
      "id": "psql",
      "owner": "build-platform",
      "reason": "fixture"
    },
    {
      "id": "sqlx",
      "owner": "build-platform",
      "reason": "fixture"
    }
  ],
"@
    }
    $postgresRequiredServiceEntry = if ($MissingRegisteredRequiredService) {
        @'
    {
      "id": "redis",
      "owner": "build-platform",
      "reason": "fixture"
    }
'@
    } else {
        @'
    {
      "id": "postgres",
      "owner": "build-platform",
      "reason": "fixture"
    }
'@
    }
    $duplicateRequiredServiceEntry = if ($DuplicateRequiredServiceRegistry) {
        @'
    {
      "id": "postgres",
      "owner": "build-platform",
      "reason": "fixture duplicate"
    },
'@
    } else {
        ""
    }
    $registeredRequiredServices = if ($MissingRequiredServiceRegistry) {
        ""
    } else {
        @"
  "required_service_registry": [
$duplicateRequiredServiceEntry$postgresRequiredServiceEntry
  ],
"@
    }
    $plannedExitTargetStateEntry = if ($MissingRegisteredExitTargetState) {
        @'
    {
      "id": "archived",
      "owner": "build-platform",
      "reason": "fixture"
    }
'@
    } else {
        @'
    {
      "id": "planned",
      "owner": "build-platform",
      "reason": "fixture"
    }
'@
    }
    $duplicateExitTargetStateEntry = if ($DuplicateExitTargetStateRegistry) {
        @'
    {
      "id": "planned",
      "owner": "build-platform",
      "reason": "fixture duplicate"
    },
'@
    } else {
        ""
    }
    $registeredExitTargetStates = if ($MissingExitTargetStateRegistry) {
        ""
    } else {
        @"
  "exit_target_state_registry": [
    {
      "id": "available",
      "owner": "build-platform",
      "reason": "fixture"
    },
$duplicateExitTargetStateEntry$plannedExitTargetStateEntry  ],
"@
    }
    $blockedTransitionExitStateEntry = if ($MissingRegisteredTransitionExitState) {
        ""
    } else {
        @'
    {
      "id": "blocked",
      "owner": "build-platform",
      "reason": "fixture"
    },
'@
    }
    $duplicateTransitionExitStateEntry = if ($DuplicateTransitionExitStateRegistry) {
        @'
    {
      "id": "blocked",
      "owner": "build-platform",
      "reason": "fixture duplicate"
    },
'@
    } else {
        ""
    }
    $registeredTransitionExitStates = if ($MissingTransitionExitStateRegistry) {
        ""
    } else {
        @"
  "transition_exit_state_registry": [
$duplicateTransitionExitStateEntry$blockedTransitionExitStateEntry    {
      "id": "ready_to_retire",
      "owner": "build-platform",
      "reason": "fixture"
    }
  ],
"@
    }
    $registeredRunnerTasks = if ($MissingRunnerTaskRegistry) {
        ""
    } else {
        @"
  "runner_task_registry": [
    {
      "id": "deleted",
      "owner": "build-platform",
      "reason": "fixture",
      "required_commands": [],
      "required_services": []
    },
$duplicateRunnerTaskEntry$frontendE2eRunnerTaskEntry    {
      "id": "migration-v001-full",
      "owner": "build-platform",
      "reason": "fixture",
      "required_commands": ["pg_isready", "psql", "sqlx"],
      "required_services": ["postgres"]
    },
    {
      "id": "node-audit",
      "owner": "build-platform",
      "reason": "fixture",
      "required_commands": ["pnpm"],
      "required_services": []
    },
    {
      "id": "rust-check",
      "owner": "build-platform",
      "reason": "fixture",
      "required_commands": ["cargo"],
      "required_services": []
    },
    {
      "id": "rustfmt-check",
      "owner": "build-platform",
      "reason": "fixture",
      "required_commands": ["cargo"],
      "required_services": []
    }
  ],
"@
    }
