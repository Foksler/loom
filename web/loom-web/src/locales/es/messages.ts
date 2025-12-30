/**
 * Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 * SPDX-License-Identifier: Proprietary
 */

export const messages = {
	// Navigation
	'nav.threads': 'Hilos',
	'nav.settings': 'Configuración',

	// Thread actions
	'thread.new': 'Nuevo hilo',
	'thread.delete': 'Eliminar',
	'thread.search': 'Buscar hilos...',
	'thread.noThreads': 'No hay hilos todavía',
	'thread.loadError': 'Error al cargar hilos',

	// Agent states
	'state.waiting_for_user_input': 'Esperando entrada',
	'state.calling_llm': 'Llamando LLM',
	'state.processing_llm_response': 'Procesando respuesta',
	'state.executing_tools': 'Ejecutando herramientas',
	'state.post_tools_hook': 'Post-procesamiento',
	'state.error': 'Error',
	'state.shutting_down': 'Cerrando',

	// Tool status
	'tool.pending': 'Pendiente',
	'tool.running': 'Ejecutando',
	'tool.completed': 'Completado',
	'tool.failed': 'Fallido',

	// Connection status
	'connection.connected': 'Conectado',
	'connection.disconnected': 'Desconectado',
	'connection.connecting': 'Conectando...',
	'connection.reconnecting': 'Reconectando...',
	'connection.error': 'Error de conexión',
	'connection.clickToReconnect': 'Clic para reconectar',
	'connection.attempt': 'Intento {count}',

	// Messages
	'message.placeholder': 'Escribe un mensaje...',
	'message.send': 'Enviar',

	// Auth - App layout
	'auth.signOut': 'Cerrar sesión',
	'auth.dashboard.welcome': 'Bienvenido,',
	'auth.dashboard.protectedPage': 'Esta es una página protegida. Debes iniciar sesión para verla.',

	// General
	'general.loading': 'Cargando...',
	'general.error': 'Ocurrió un error',
	'general.retry': 'Reintentar',
	'general.cancel': 'Cancelar',
	'general.save': 'Guardar',
	'general.close': 'Cerrar',

	// Auth - Login
	'auth.login.title': 'Iniciar sesión en Loom',
	'auth.login.subtitle': 'Elige tu método de inicio de sesión preferido',
	'auth.login.github': 'Continuar con GitHub',
	'auth.login.google': 'Continuar con Google',
	'auth.login.or': 'o',
	'auth.login.emailLabel': 'Dirección de correo electrónico',
	'auth.login.emailPlaceholder': 'tu@ejemplo.com',
	'auth.login.sendMagicLink': 'Enviar enlace mágico',
	'auth.login.sending': 'Enviando...',
	'auth.login.checkEmail': 'Revisa tu correo',
	'auth.login.magicLinkSent': 'Enviamos un enlace de inicio de sesión a',
	'auth.login.useDifferentEmail': 'Usar otro correo electrónico',
	'auth.login.error': 'No se pudo enviar el enlace mágico. Por favor, inténtalo de nuevo.',

	// Auth - Device Code
	'auth.device.title': 'Autorizar CLI',
	'auth.device.subtitle': 'Ingresa el código que aparece en tu terminal para vincular tu CLI con tu cuenta',
	'auth.device.inputLabel': 'Ingresa tu código',
	'auth.device.placeholder': 'XXX-XXX-XXX',
	'auth.device.authorize': 'Autorizar',
	'auth.device.authorizing': 'Autorizando...',
	'auth.device.success': '¡Autorizado!',
	'auth.device.successMessage': 'Ahora puedes cerrar esta página y volver a tu terminal.',
	'auth.device.error': 'Código inválido o expirado. Por favor, verifica tu terminal e inténtalo de nuevo.',
	'auth.device.noCode': '¿No tienes un código? Ejecuta',
	'auth.device.cliCommand': 'loom login',
	'auth.device.inTerminal': 'en tu terminal.',

	// Settings - Sessions
	'settings.sessions.title': 'Sesiones activas',
	'settings.sessions.description': 'Administra tus sesiones activas en todos los dispositivos',
	'settings.sessions.current': 'Sesión actual',
	'settings.sessions.lastUsed': 'Último uso',
	'settings.sessions.createdAt': 'Creada',
	'settings.sessions.revoke': 'Revocar',
	'settings.sessions.revokeConfirm': '¿Estás seguro de que deseas revocar esta sesión?',
	'settings.sessions.noSessions': 'No hay sesiones activas',
	'settings.sessions.web': 'Web',
	'settings.sessions.cli': 'CLI',
	'settings.sessions.vscode': 'VS Code',

	// Settings - Navigation
	'settings.nav.sessions': 'Sesiones',
	'settings.nav.profile': 'Perfil',
	'settings.nav.orgs': 'Organizaciones',

	// Settings - Profile
	'settings.profile.title': 'Configuración del Perfil',
	'settings.profile.displayName': 'Nombre para mostrar',
	'settings.profile.email': 'Correo electrónico',
	'settings.profile.emailHint': 'El correo no se puede cambiar aquí',
	'settings.profile.locale': 'Idioma',
	'settings.profile.save': 'Guardar Cambios',
	'settings.profile.saving': 'Guardando...',
	'settings.profile.saved': 'Perfil actualizado correctamente',
	'settings.profile.error': 'Error al actualizar el perfil',

	// Organizations
	'orgs.title': 'Organizaciones',
	'orgs.description': 'Gestiona tus organizaciones y equipos',
	'orgs.create': 'Crear Organización',
	'orgs.createTitle': 'Crear Nueva Organización',
	'orgs.name': 'Nombre de la Organización',
	'orgs.slug': 'Slug de URL',
	'orgs.slugHint': 'Usado en URLs, solo letras minúsculas y guiones',
	'orgs.visibility': 'Visibilidad',
	'orgs.visibilityPublic': 'Pública',
	'orgs.visibilityPrivate': 'Privada',
	'orgs.joinPolicy': 'Política de Acceso',
	'orgs.joinPolicyOpen': 'Abierta',
	'orgs.joinPolicyRequest': 'Solicitar Acceso',
	'orgs.joinPolicyInvite': 'Solo por Invitación',
	'orgs.members': 'Miembros',
	'orgs.teams': 'Equipos',
	'orgs.apiKeys': 'Claves API',
	'orgs.settings': 'Configuración',
	'orgs.noOrgs': 'Aún no hay organizaciones',
	'orgs.delete': 'Eliminar Organización',
	'orgs.deleteConfirm': '¿Estás seguro de que quieres eliminar esta organización?',

	// Settings - Organizations
	'settings.orgs.title': 'Organizaciones',
	'settings.orgs.description': 'Gestiona tus organizaciones y membresías de equipo',
	'settings.orgs.create': 'Crear Organización',
	'settings.orgs.noOrgs': 'Aún no hay organizaciones',
	'settings.orgs.loadError': 'Error al cargar organizaciones',
	'settings.orgs.personal': 'Personal',
	'settings.orgs.slug': 'Slug',
	'settings.orgs.member': 'miembro',
	'settings.orgs.members': 'miembros',
	'settings.orgs.visibility.public': 'Pública',
	'settings.orgs.visibility.unlisted': 'No listada',
	'settings.orgs.visibility.private': 'Privada',

	// Settings - Organizations - New
	'settings.orgs.new.title': 'Crear Organización',
	'settings.orgs.new.name': 'Nombre de la Organización',
	'settings.orgs.new.namePlaceholder': 'Mi Organización',
	'settings.orgs.new.slug': 'Slug',
	'settings.orgs.new.slugPlaceholder': 'mi-organizacion',
	'settings.orgs.new.slugHint': 'Identificador seguro para URL (letras minúsculas, números y guiones)',
	'settings.orgs.new.visibility': 'Visibilidad',
	'settings.orgs.new.create': 'Crear Organización',
	'settings.orgs.new.creating': 'Creando...',
	'settings.orgs.new.error': 'Error al crear la organización',
	'settings.orgs.new.requiredFields': 'El nombre y el slug son obligatorios',

	// Members
	'members.title': 'Miembros',
	'members.add': 'Agregar Miembro',
	'members.remove': 'Eliminar',
	'members.removeConfirm': '¿Estás seguro de que quieres eliminar a este miembro?',
	'members.role': 'Rol',
	'members.roleOwner': 'Propietario',
	'members.roleAdmin': 'Administrador',
	'members.roleMember': 'Miembro',
	'members.changeRole': 'Cambiar Rol',
	'members.noMembers': 'Aún no hay miembros',

	// Teams
	'teams.title': 'Equipos',
	'teams.create': 'Crear Equipo',
	'teams.createTitle': 'Crear Nuevo Equipo',
	'teams.name': 'Nombre del Equipo',
	'teams.slug': 'Slug de URL',
	'teams.members': 'Miembros del Equipo',
	'teams.addMember': 'Agregar Miembro',
	'teams.removeMember': 'Eliminar del Equipo',
	'teams.noTeams': 'Aún no hay equipos',
	'teams.delete': 'Eliminar Equipo',
	'teams.deleteConfirm': '¿Estás seguro de que quieres eliminar este equipo?',
	'teams.roleMaintainer': 'Mantenedor',
	'teams.roleMember': 'Miembro',

	// API Keys
	'apiKeys.title': 'Claves API',
	'apiKeys.description': 'Gestiona las claves API para acceso programático',
	'apiKeys.create': 'Crear Clave API',
	'apiKeys.createTitle': 'Crear Nueva Clave API',
	'apiKeys.name': 'Nombre de la Clave',
	'apiKeys.nameHint': 'Un nombre descriptivo para esta clave',
	'apiKeys.scopes': 'Permisos',
	'apiKeys.scopesHint': 'Selecciona los permisos para esta clave',
	'apiKeys.scopeThreadsRead': 'Leer Conversaciones',
	'apiKeys.scopeThreadsWrite': 'Escribir Conversaciones',
	'apiKeys.scopeLlmUse': 'Usar LLM',
	'apiKeys.prefix': 'Prefijo de Clave',
	'apiKeys.createdAt': 'Creada',
	'apiKeys.lastUsed': 'Último Uso',
	'apiKeys.never': 'Nunca',
	'apiKeys.revoke': 'Revocar',
	'apiKeys.revokeConfirm': '¿Estás seguro de que quieres revocar esta clave API?',
	'apiKeys.noKeys': 'Aún no hay claves API',
	'apiKeys.copyWarning': 'Copia esta clave ahora. No podrás verla de nuevo.',
	'apiKeys.copied': 'Copiado al portapapeles',
};
