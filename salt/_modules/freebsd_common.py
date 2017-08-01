def sysrc(value):
    """Call sysrc.
    CLI Example:

    .. code-block:: bash

        salt '*' freebsd_common.sysrc sshd_enable=YES
        salt '*' freebsd_common.sysrc static_routes
    """
    return __salt__['cmd.run_all']("sysrc %s" % value)
