// React component
import React from 'react';

function UserList({ users, onSelect }) {
  return (
    <div>
      {users.map(user => (
        <UserItem
          key={user.id}
          user={user}
          style={{ margin: 10, padding: 5 }}
          onClick={() => onSelect(user.id)}
        />
      ))}
    </div>
  );
}

export default UserList;
