def ensure [path: string, field: string, body: record] { }

def main [] {
    ensure remotepathmapping host {
        host: torrents
        remotePath: /var/lib/
        localPath: /tank/media/
    }
}
