/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

export const messages = {
	// Navigation
	'nav.threads': 'Threads',
	'nav.weavers': 'Weavers',
	'nav.settings': 'Settings',
	'nav.admin': 'Admin',
	'nav.jobs': 'Jobs',

	// Thread actions
	'thread.new': 'New Thread',
	'thread.delete': 'Delete',
	'thread.search': 'Search threads...',
	'thread.noThreads': 'No threads yet',
	'thread.loadError': 'Failed to load threads',

	// Agent states
	'state.waiting_for_user_input': 'Waiting for input',
	'state.calling_llm': 'Calling LLM',
	'state.processing_llm_response': 'Processing response',
	'state.executing_tools': 'Executing tools',
	'state.post_tools_hook': 'Post-processing',
	'state.error': 'Error',
	'state.shutting_down': 'Shutting down',

	// Tool status
	'tool.pending': 'Pending',
	'tool.running': 'Running',
	'tool.completed': 'Completed',
	'tool.failed': 'Failed',

	// Connection status
	'connection.connected': 'Connected',
	'connection.disconnected': 'Disconnected',
	'connection.connecting': 'Connecting...',
	'connection.reconnecting': 'Reconnecting...',
	'connection.error': 'Connection error',
	'connection.clickToReconnect': 'Click to reconnect',
	'connection.attempt': 'Attempt {count}',

	// Messages
	'message.placeholder': 'Type a message...',
	'message.send': 'Send',

	// Auth - App layout
	'auth.signOut': 'Sign out',
	'auth.dashboard.welcome': 'Welcome,',
	'auth.dashboard.protectedPage': 'This is a protected page. You must be logged in to see this.',

	// General
	'general.loading': 'Loading...',
	'general.error': 'An error occurred',
	'general.retry': 'Retry',
	'general.cancel': 'Cancel',
	'general.save': 'Save',
	'general.close': 'Close',

	// Auth - Login
	'auth.login.title': 'Sign in to Loom',
	'auth.login.subtitle': 'Choose your preferred sign-in method',
	'auth.login.github': 'Continue with GitHub',
	'auth.login.google': 'Continue with Google',
	'auth.login.or': 'or',
	'auth.login.emailLabel': 'Email address',
	'auth.login.emailPlaceholder': 'you@example.com',
	'auth.login.sendMagicLink': 'Send magic link',
	'auth.login.sending': 'Sending...',
	'auth.login.checkEmail': 'Check your email',
	'auth.login.magicLinkSent': 'We sent a sign-in link to',
	'auth.login.useDifferentEmail': 'Use a different email',
	'auth.login.error': 'Unable to send magic link. Please try again.',

	// Auth - Device Code
	'auth.device.title': 'Authorize CLI',
	'auth.device.subtitle': 'Enter the code displayed in your terminal to link your CLI with your account',
	'auth.device.inputLabel': 'Enter your code',
	'auth.device.placeholder': 'XXX-XXX-XXX',
	'auth.device.authorize': 'Authorize',
	'auth.device.authorizing': 'Authorizing...',
	'auth.device.success': 'Authorized!',
	'auth.device.successMessage': 'You can now close this page and return to your terminal.',
	'auth.device.error': 'Invalid or expired code. Please check your terminal and try again.',
	'auth.device.noCode': "Don't have a code? Run",
	'auth.device.cliCommand': 'loom login',
	'auth.device.inTerminal': 'in your terminal.',

	// Settings - Sessions
	'settings.sessions.title': 'Active Sessions',
	'settings.sessions.description': 'Manage your active sessions across devices',
	'settings.sessions.current': 'Current session',
	'settings.sessions.lastUsed': 'Last used',
	'settings.sessions.createdAt': 'Created',
	'settings.sessions.revoke': 'Revoke',
	'settings.sessions.revokeConfirm': 'Are you sure you want to revoke this session?',
	'settings.sessions.noSessions': 'No active sessions',
	'settings.sessions.web': 'Web',
	'settings.sessions.cli': 'CLI',
	'settings.sessions.vscode': 'VS Code',

	// Settings - Navigation
	'settings.nav.sessions': 'Sessions',
	'settings.nav.profile': 'Profile',
	'settings.nav.orgs': 'Organizations',

	// Settings - Profile
	'settings.profile.title': 'Profile Settings',
	'settings.profile.displayName': 'Display Name',
	'settings.profile.email': 'Email',
	'settings.profile.emailHint': 'Email cannot be changed here',
	'settings.profile.locale': 'Language',
	'settings.profile.save': 'Save Changes',
	'settings.profile.saving': 'Saving...',
	'settings.profile.saved': 'Profile updated successfully',
	'settings.profile.error': 'Failed to update profile',

	// Organizations
	'orgs.title': 'Organizations',
	'orgs.description': 'Manage your organizations and teams',
	'orgs.create': 'Create Organization',
	'orgs.createTitle': 'Create New Organization',
	'orgs.name': 'Organization Name',
	'orgs.slug': 'URL Slug',
	'orgs.slugHint': 'Used in URLs, lowercase letters and hyphens only',
	'orgs.visibility': 'Visibility',
	'orgs.visibilityPublic': 'Public',
	'orgs.visibilityPrivate': 'Private',
	'orgs.joinPolicy': 'Join Policy',
	'orgs.joinPolicyOpen': 'Open',
	'orgs.joinPolicyRequest': 'Request to Join',
	'orgs.joinPolicyInvite': 'Invite Only',
	'orgs.members': 'Members',
	'orgs.teams': 'Teams',
	'orgs.apiKeys': 'API Keys',
	'orgs.settings': 'Settings',
	'orgs.noOrgs': 'No organizations yet',
	'orgs.delete': 'Delete Organization',
	'orgs.deleteConfirm': 'Are you sure you want to delete this organization?',

	// Settings - Organizations
	'settings.orgs.title': 'Organizations',
	'settings.orgs.description': 'Manage your organizations and team memberships',
	'settings.orgs.create': 'Create Organization',
	'settings.orgs.noOrgs': 'No organizations yet',
	'settings.orgs.loadError': 'Failed to load organizations',
	'settings.orgs.personal': 'Personal',
	'settings.orgs.slug': 'Slug',
	'settings.orgs.member': 'member',
	'settings.orgs.members': 'members',
	'settings.orgs.visibility.public': 'Public',
	'settings.orgs.visibility.unlisted': 'Unlisted',
	'settings.orgs.visibility.private': 'Private',

	// Settings - Organizations - New
	'settings.orgs.new.title': 'Create Organization',
	'settings.orgs.new.name': 'Organization Name',
	'settings.orgs.new.namePlaceholder': 'My Organization',
	'settings.orgs.new.slug': 'Slug',
	'settings.orgs.new.slugPlaceholder': 'my-organization',
	'settings.orgs.new.slugHint': 'URL-safe identifier (lowercase letters, numbers, and hyphens)',
	'settings.orgs.new.visibility': 'Visibility',
	'settings.orgs.new.create': 'Create Organization',
	'settings.orgs.new.creating': 'Creating...',
	'settings.orgs.new.error': 'Failed to create organization',
	'settings.orgs.new.requiredFields': 'Name and slug are required',

	// Members
	'members.title': 'Members',
	'members.add': 'Add Member',
	'members.remove': 'Remove',
	'members.removeConfirm': 'Are you sure you want to remove this member?',
	'members.role': 'Role',
	'members.roleOwner': 'Owner',
	'members.roleAdmin': 'Admin',
	'members.roleMember': 'Member',
	'members.changeRole': 'Change Role',
	'members.noMembers': 'No members yet',

	// Teams
	'teams.title': 'Teams',
	'teams.create': 'Create Team',
	'teams.createTitle': 'Create New Team',
	'teams.name': 'Team Name',
	'teams.slug': 'URL Slug',
	'teams.members': 'Team Members',
	'teams.addMember': 'Add Member',
	'teams.removeMember': 'Remove',
	'teams.removeMemberConfirm': 'Are you sure you want to remove this member from the team?',
	'teams.noTeams': 'No teams yet',
	'teams.noMembers': 'No members in this team',
	'teams.delete': 'Delete Team',
	'teams.deleteConfirm': 'Are you sure you want to delete this team?',
	'teams.roleMaintainer': 'Maintainer',
	'teams.roleMember': 'Member',
	'teams.role.maintainer': 'Maintainer',
	'teams.role.member': 'Member',
	'teams.backToOrg': 'Back to organization',
	'teams.changeRole': 'Change role',
	'teams.settings': 'Team Settings',
	'teams.teamName': 'Team Name',
	'teams.save': 'Save Changes',
	'teams.saving': 'Saving...',
	'teams.saved': 'Team updated successfully',
	'teams.deleteTeam': 'Delete Team',
	'teams.deleteTeamConfirm': 'Are you sure you want to delete this team? This action cannot be undone.',
	'teams.deleteTeamWarning': 'This will permanently delete the team and remove all members.',
	'teams.deleting': 'Deleting...',
	'teams.selectMember': 'Select a member to add',
	'teams.selectRole': 'Select role',
	'teams.adding': 'Adding...',
	'teams.add': 'Add',
	'teams.noAvailableMembers': 'No organization members available to add',

	// API Keys
	'apiKeys.title': 'API Keys',
	'apiKeys.description': 'Manage API keys for programmatic access',
	'apiKeys.create': 'Create API Key',
	'apiKeys.createTitle': 'Create New API Key',
	'apiKeys.name': 'Key Name',
	'apiKeys.nameHint': 'A descriptive name for this key',
	'apiKeys.scopes': 'Scopes',
	'apiKeys.scopesHint': 'Select the permissions for this key',
	'apiKeys.scopeThreadsRead': 'Read Threads',
	'apiKeys.scopeThreadsWrite': 'Write Threads',
	'apiKeys.scopeLlmUse': 'Use LLM',
	'apiKeys.prefix': 'Key Prefix',
	'apiKeys.createdAt': 'Created',
	'apiKeys.lastUsed': 'Last Used',
	'apiKeys.never': 'Never',
	'apiKeys.revoke': 'Revoke',
	'apiKeys.revokeConfirm': 'Are you sure you want to revoke this API key?',
	'apiKeys.noKeys': 'No API keys yet',
	'apiKeys.copyWarning': "Copy this key now. You won't be able to see it again.",
	'apiKeys.copied': 'Copied to clipboard',

	// General - Pagination
	'general.search': 'Search',
	'general.previous': 'Previous',
	'general.next': 'Next',
	'general.done': 'Done',
	'general.refresh': 'Refresh',

	// Admin - Users
	'admin.users.title': 'User Management',
	'admin.users.description': 'View and manage users across the platform',
	'admin.users.searchPlaceholder': 'Search by name or email...',
	'admin.users.empty': 'No users found',
	'admin.users.created': 'Created',
	'admin.users.lastLogin': 'Last login',
	'admin.users.impersonate': 'Impersonate',

	// Admin - Impersonation
	'admin.impersonation.banner': 'Admin {admin} impersonating {user}',
	'admin.impersonation.stop': 'Stop Impersonating',

	// Admin - Anthropic Accounts
	'admin.anthropic.title': 'Claude Max Accounts',
	'admin.anthropic.add_account': 'Add Account',
	'admin.anthropic.remove': 'Remove',
	'admin.anthropic.remove_confirm': 'Remove this account from the pool?',
	'admin.anthropic.status.available': 'Available',
	'admin.anthropic.status.cooling_down': 'Cooling Down',
	'admin.anthropic.status.disabled': 'Disabled',
	'admin.anthropic.cooldown_remaining': '{time} remaining',
	'admin.anthropic.expires_at': 'Token expires: {time}',
	'admin.anthropic.no_accounts': 'No accounts configured. Add a Claude Max account to get started.',
	'admin.anthropic.not_configured': 'Anthropic OAuth pool is not configured on this server.',

	// Weavers
	'weavers.title': 'Weavers',
	'weavers.description': 'Manage ephemeral development environments',
	'weavers.create': 'Create Weaver',
	'weavers.createFirst': 'Create your first weaver',
	'weavers.createTitle': 'Create New Weaver',
	'weavers.empty': 'No weavers running',
	'weavers.delete': 'Delete',
	'weavers.deleteConfirm': 'Are you sure you want to delete this weaver?',
	'weavers.image': 'Image',
	'weavers.imageName': 'Container Image',
	'weavers.created': 'Created',
	'weavers.age': 'Age',
	'weavers.lifetime': 'Lifetime',
	'weavers.lifetimeLabel': 'Lifetime (TTL)',
	'weavers.workdir': 'Working Directory',
	'weavers.hour': 'hour',
	'weavers.hours': 'hours',
	'weavers.attach': 'Attach',
	'weavers.backToList': 'Back to Weavers',
	'weavers.notFound': 'Weaver not found',
	'weavers.logs': 'Logs',
	'weavers.logsTitle': 'Weaver Logs',
	'weavers.logsConnecting': 'Connecting to log stream...',
	'weavers.logsNoData': 'No logs available yet',
	'weavers.logsError': 'Failed to connect to log stream',
	'weavers.logsClosed': 'Log stream closed',
	'weavers.creatingTitle': 'Creating Weaver',
	'weavers.creatingProgress': 'Provisioning your weaver environment...',
	'weavers.creatingWait': 'Waiting for weaver to become ready. You will be redirected automatically.',
	'weavers.createFailed': 'Weaver creation failed',
	'weavers.createTimeout': 'Weaver creation timed out',

	// Weavers - Terminal
	'weavers.terminal.connected': 'Connected',
	'weavers.terminal.connecting': 'Connecting...',
	'weavers.terminal.disconnected': 'Disconnected',
	'weavers.terminal.error': 'Connection error',
	'weavers.terminal.reconnect': 'Reconnect',
	'weavers.terminal.pending': 'Weaver is starting...',
	'weavers.terminal.pendingHint': 'The terminal will be available once the weaver is running',
	'weavers.terminal.notRunning': 'Weaver is not running (status: {status})',

	// Jobs - Admin page
	'jobs.title': 'Background Jobs',
	'jobs.description': 'Monitor and manage scheduled background jobs',
	'jobs.loading': 'Loading jobs...',
	'jobs.runNow': 'Run Now',
	'jobs.running': 'Running...',
	'jobs.history': 'History',
	'jobs.hideHistory': 'Hide History',
	'jobs.viewHistory': 'View History',
	'jobs.noJobs': 'No background jobs configured',
	'jobs.never': 'Never run',
	'jobs.lastRun': 'Last run',
	'jobs.duration': 'Duration',
	'jobs.interval': 'Interval',
	'jobs.type': 'Type',
	'jobs.failures': '{count} consecutive failures',

	// Jobs - Status
	'jobs.status.healthy': 'Healthy',
	'jobs.status.degraded': 'Degraded',
	'jobs.status.unhealthy': 'Unhealthy',
	'jobs.status.succeeded': 'Succeeded',
	'jobs.status.failed': 'Failed',
	'jobs.status.running': 'Running',
	'jobs.status.cancelled': 'Cancelled',

	// Jobs - Trigger sources
	'jobs.trigger.schedule': 'Schedule',
	'jobs.trigger.manual': 'Manual',
	'jobs.trigger.retry': 'Retry',

	// Jobs - History
	'jobs.history.title': 'Run History',
	'jobs.history.status': 'Status',
	'jobs.history.started': 'Started',
	'jobs.history.duration': 'Duration',
	'jobs.history.trigger': 'Trigger',
	'jobs.history.details': 'Details',
	'jobs.history.loadMore': 'Load More',
	'jobs.history.noRuns': 'No run history available',

	// Jobs - Time formatting
	'jobs.time.justNow': 'just now',
	'jobs.time.minutesAgo': '{count} min ago',
	'jobs.time.hoursAgo': '{count} hours ago',
	'jobs.time.daysAgo': '{count} days ago',

	// Repos - Header
	'client.repos.header.clone': 'Clone',
	'client.repos.header.clone_https': 'Clone with HTTPS',
	'client.repos.header.copied': 'Copied!',
	'client.repos.header.copy': 'Copy',
	'client.repos.header.default_branch': 'Default branch',

	// Repos - Navigation
	'client.repos.nav.code': 'Code',
	'client.repos.nav.commits': 'Commits',
	'client.repos.nav.branches': 'Branches',
	'client.repos.nav.settings': 'Settings',

	// Repos - Tree view
	'client.repos.tree.empty': 'This directory is empty',

	// Repos - Blob view
	'client.repos.blob.lines': 'lines',
	'client.repos.blob.bytes': 'bytes',
	'client.repos.blob.blame': 'Blame',
	'client.repos.blob.copy': 'Copy',
	'client.repos.blob.copied': 'Copied!',
	'client.repos.blob.raw': 'Raw',
	'client.repos.blob.binary_not_shown': 'Binary file not shown',

	// Repos - Weaver
	'client.repos.weaver.open': 'Open in Weaver',
	'client.repos.weaver.create_failed': 'Failed to create weaver',

	// Repos - Branch selector
	'client.repos.branch.find': 'Find a branch...',
	'client.repos.branch.not_found': 'No branches found',
	'client.repos.branch.default': 'default',

	// Repos - Commits
	'client.repos.commits.minutes_ago': '{count} minutes ago',
	'client.repos.commits.hours_ago': '{count} hours ago',
	'client.repos.commits.days_ago': '{count} days ago',
	'client.repos.commits.committed': 'committed',
	'client.repos.commits.copy_sha': 'Copy SHA',
	'client.repos.commits.empty': 'No commits found',

	// Repos - Diff
	'client.repos.diff.parent': 'Parent',
	'client.repos.diff.parents': 'Parents',
	'client.repos.diff.showing': 'Showing',
	'client.repos.diff.changed_file': 'changed file',
	'client.repos.diff.changed_files': 'changed files',

	// Repos - Blame
	'client.repos.blame.lines': 'lines',
	'client.repos.blame.today': 'today',
	'client.repos.blame.days_ago': '{count}d ago',
	'client.repos.blame.months_ago': '{count}mo ago',
	'client.repos.blame.years_ago': '{count}y ago',
	'client.repos.blame.empty': 'No blame information available',

	// Repos - Compare
	'client.repos.compare.ahead': 'ahead',
	'client.repos.compare.behind': 'behind',
	'client.repos.compare.commit': 'commit',
	'client.repos.compare.commits': 'commits',
	'client.repos.compare.commits_heading': 'Commits',
	'client.repos.compare.identical': 'These branches are identical',
};
