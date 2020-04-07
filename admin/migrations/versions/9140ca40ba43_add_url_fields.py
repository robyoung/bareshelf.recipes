"""add url fields

Revision ID: 9140ca40ba43
Revises: 0e04296319d3
Create Date: 2020-04-07 19:36:57.361691

"""
from alembic import op
import sqlalchemy as sa


# revision identifiers, used by Alembic.
revision = '9140ca40ba43'
down_revision = '0e04296319d3'
branch_labels = None
depends_on = None


def upgrade():
    op.add_column('ingredient', sa.Column('url', sa.String(), nullable=False))
    op.create_index(op.f('ix_ingredient_url'), 'ingredient', ['url'], unique=True)
    op.add_column('recipe', sa.Column('url', sa.String(), nullable=False))
    op.create_index(op.f('ix_recipe_url'), 'recipe', ['url'], unique=True)


def downgrade():
    op.drop_index(op.f('ix_recipe_url'), table_name='recipe')
    op.drop_column('recipe', 'url')
    op.drop_index(op.f('ix_ingredient_url'), table_name='ingredient')
    op.drop_column('ingredient', 'url')
