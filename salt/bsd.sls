py27-salt:
  pkg.latest:
    - name: py27-salt
  service.running:
  - names:
    - salt_minion
  - require:
    - pkg: py27-salt
  - watch:
    - file: /usr/local/etc/salt/minion

/usr/local/etc/salt/minion: file.exists
