#!/bin/bash
mongo -- "$MONGO_INITDB_DATABASE" <<EOF
    var rootUser = '$MONGO_INITDB_ROOT_USERNAME';
    var rootPassword = '$MONGO_INITDB_ROOT_PASSWORD';
    var admin = db.getSiblingDB('admin');
    admin.auth(rootUser, rootPassword);

    db = db.getSiblingDB('$MONGO_INITDB_DATABASE');
    var user = '$MONGO_INITDB_USERNAME';
    var passwd = '$MONGO_INITDB_PASSWORD';
    db.createUser({
        user: user,
        pwd: passwd,
        roles: [
            {
                role: "dbOwner",
                db: '$MONGO_INITDB_DATABASE'
            }
        ]
    });
    db.createCollection('wd_entities');
EOF