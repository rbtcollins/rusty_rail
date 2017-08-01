# Derived from the Salt codebase (due to implementing the ip interface); that
# is Apache-2 so license compatible.

import logging

import salt.utils.validate.net

# Define the module's virtual name
__virtualname__ = 'ip'

# From rh_ip.py
_CONFIG_TRUE = ['yes', 'on', 'true', '1', True]
_CONFIG_FALSE = ['no', 'off', 'false', '0', False]
_IFACE_TYPES = [
    'gre', 'alias'
]


log = logging.getLogger(__name__)
# TODO: error handling throughout.


def __virtual__():
    '''Confine this module to FreeBSD based distros'''
    if __grains__['os'] == 'FreeBSD':
        return __virtualname__
    return False


def _raise_error(message):
    log.error(message)
    raise AttributeError(message)


def build_interface(iface, iface_type, enabled, **settings):
    '''Build an interface script for a network interface.

    CLI Example:

    .. code-block:: bash

        salt '*' ip.build_interface eth0 eth <settings>
        salt '*' ip.build_interface eth0.0 alias <settings>
    '''
    iface = iface.lower()
    iface_type = iface_type.lower()

    if iface_type not in _IFACE_TYPES:
        _raise_error(
            "Bad interface type %s not known in %s" % (iface_type, _IFACE_TYPES))

    opts = _parse_int_settings(settings, iface_type, enabled, iface)
    # Could read in current value here for retain-settings
    # XXX: should be cross cutting scatter-gather across all interfaces?
    cloned_interfaces = set()
    if iface_type in ['eth']:
        ifcfg = 'ifconfig_%(major)s="inet %(ipaddr)s netmask %(netmask)s"' % opts
    elif iface_type in ["alias"]:
        ifcfg = 'ifconfig_%(major)s_alias%(minor)s="inet %(ipaddr)s netmask %(netmask)s"' % opts
    elif iface_type in ["gre"]:
        ifcfg = 'ifconfig_%(major)s="inet %(ipaddr)s %(peer_inner_addr)s netmask %(netmask)s tunnel %(tunnel_addr)s %(tunnel_peer)s"' % opts
        cloned_interfaces.add(opts['major'])
    else:
        _raise_error("Unexpected interface type %s" % iface_type)

    if 'test' in settings and settings['test']:
        return [ifcfg]

    __salt__['freebsd_common.sysrc'](ifcfg)
    if cloned_interfaces:
        cloned_cfg = 'cloned_interfaces="%s"' % ' '.join(cloned_interfaces)
        __salt__['freebsd_common.sysrc'](cloned_cfg)
    return [ifcfg]


def build_network_settings(**settings):
    '''Build the global network script.'''
    # Not implemented yet
    return ''


def get_interface(iface):
    '''Return the contents of an interface script
    
    Mocked out to get minimal virtual IP management working.
    '''
    # Not implemented yet
    return ''


def get_network_settings():
    '''Return the contents of the global network script.

    Mocked to get virtual IP management working.
    '''
    return ''


def up(iface, iface_type):
    '''Start up a network interface

    CLI Example:

    .. code-block:: bash

        salt '*' ip.up eth0
    '''
    if '.' not in iface:
        major = iface
    else:
        major = iface[:iface.find('.')]
    return __salt__['cmd.run']('/etc/rc.d/netif restart %s' % major)


def down(iface, iface_type):
    '''Take an interface down.'''
    if '.' not in iface:
        major = iface
    else:
        major = iface[:iface.find('.')]
    return __salt__['cmd.run']('ifconfig %s down' % major)


def apply_network_settings(**settings):
    """Apply global network configuration."""
    if 'require_reboot' not in settings:
        settings['require_reboot'] = False
    if 'apply_hostname' not in settings:
        settings['apply_hostname'] = False

    hostname_res = True
    if settings['apply_hostname'] in _CONFIG_TRUE:
        if 'hostname' in settings:
            hostname_res = __salt__['network.mod_hostname'](settings['hostname'])
        else:
            log.warning(
                'The network state sls is trying to apply hostname '
                'changes but no hostname is defined.'
            )
            hostname_res = False

    res = True
    if settings['require_reboot'] in _CONFIG_TRUE:
        log.warning(
            'The network state sls is requiring a reboot of the system to '
            'properly apply network configuration.'
        )
        res = True
    else:
        res = __salt__['cmd.run']('/etc/netstart restart')

    return hostname_res and res


def get_routes(iface):
    """Find the routes for an interface."""
    # 1: get the static_routes entry.
    res = __salt__['freebsd_common.sysrc']("static_routes")
    output = res["stdout"]
    if res["retcode"] or not output.startswith("static_routes:"):
        _raise_error(
            "Invalid sysrc output %r" % (res,))
    routekeys = output[len("static_routes:"):].strip().split()
    if not routekeys or routekeys == ['NO']:
        return []
    result = ['static_routes+="']
    for key in routekeys:
        route_iface, line = _get_static_route(key)
        if route_iface != iface:
            continue
        result.append(line)
        result[0] += " " + key
    result[0] += '"'
    return result


def build_routes(iface, **settings):
    """Build/apply routes for iface."""
    iface = iface.lower()
    settings = dict((k.lower(), v) for (k, v) in settings.items())
    if 'routes' not in settings:
        _raise_error("No routes supplied for " + iface)
    routes = []
    for route in settings['routes']:
        # Required options: name, ipaddr, netmask, gateway
        routes.append({
            'name': route['name'],
            'ipaddr': route['ipaddr'],
            'netmask': route['netmask'],
            'gateway': route['gateway']
            })
    routekeys = set()
    static_line = 'static_routes+="'
    routelines = []
    for route in routes:
        if route['name'] in routekeys:
            _raise_error("Duplicate route " + route['name'])
        static_line += " %s:%s" % (route['name'], iface)
        routekeys.add(route['name'])
        routelines.append('route_%s="%s %s %s"' % (
            route['name'], route['ipaddr'], route['gateway'], route['netmask']
            ))
    static_line += '"'
    if settings['test']:
        return [static_line] + routelines

    # Serialise route entries.
    for routeline in routelines:
        __salt__['freebsd_common.sysrc'](routeline)
    # And the root keys
    __salt__['freebsd_common.sysrc'](static_line)
    return [static_line] + routelines


def _get_static_route(key):
    if ':' in key:
        name, iface = key.split(':')
    else:
        name, iface = key, None
    syskey = "route_" + name
    res = __salt__['freebsd_common.sysrc'](syskey)
    if res["retcode"] == 1:
        return ""
    if res["retcode"]:
        _raise_error("Failed to get route %r" % syskey)
    return iface, res["stdout"]


def _parse_int_settings(opts, iface_type, enabled, iface):
    '''
    Filters given options and outputs valid settings for a
    network interface.
    '''
    result = {'name': iface}
    if '.' in iface:
        result['major'] = iface[:iface.find('.')]
        result['minor'] = iface[iface.find('.')+1:]
    else:
        result['major'] = iface
        result['minor'] = None

    for opt in ['ipaddr', 'master', 'netmask', 'srcaddr', 'delay', 'domain', 'gateway', 'zone']:
        if opt in opts:
            result[opt] = opts[opt]

    result.update(_parse_type[iface_type](iface, opts))
    return result


def _parse_gre(iface, opts):
    """Parse GRE specific options:

     - peer_inner_addr: the far end address to route packets to.
     - tunnel_addr: the address to send/receive GRE packets at.
     - tunnel_peer: the address to send GRE packets to.
    """
    result = {}
    result.update(_require_opt('peer_inner_addr', iface, opts))
    result.update(_require_opt('tunnel_addr', iface, opts))
    result.update(_require_opt('tunnel_peer', iface, opts))
    return result


def _parse_alias(iface, opts):
    return {}


_parse_type = {'gre': _parse_gre, 'alias': _parse_alias}


def _require_opt(optname, iface, opts):
    if optname not in opts:
        _raise_error(
            "option %r not supplied for interface %s" % (optname, iface))
    return {optname: opts[optname]}
